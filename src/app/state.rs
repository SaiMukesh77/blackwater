use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use egui::TextureHandle;
use crate::models::{BatchStats, PhotoSource, ProcessStatus, ProcessedPhoto, ScannedItem};
use crate::processor::queue::{ProcessJob, WorkerEvent};
use crate::processor::run_worker_pool;
use crate::scanner::{run_scanner, ScanEvent};
use crate::settings::{get_process_memory_usage, get_total_physical_ram, AppSettings};

/// Primary active screen in the workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    Start,
    Scanning,
    FormatSelect,
    Processing,
    Result,
    Settings,
}

/// Thumbnail LRU Cache to keep memory bounded (O(1) texture memory).
pub struct ThumbnailCache {
    max_entries: usize,
    order: VecDeque<String>,
    textures: std::collections::HashMap<String, TextureHandle>,
}

impl ThumbnailCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            order: VecDeque::new(),
            textures: std::collections::HashMap::new(),
        }
    }

    pub fn set_capacity(&mut self, max: usize) {
        self.max_entries = max;
        while self.order.len() > self.max_entries {
            if let Some(oldest_key) = self.order.pop_front() {
                self.textures.remove(&oldest_key);
            }
        }
    }

    pub fn get(&mut self, key: &str) -> Option<&TextureHandle> {
        if self.textures.contains_key(key) {
            // Move to end (most recently used)
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                let removed = self.order.remove(pos).unwrap();
                self.order.push_back(removed);
            }
            self.textures.get(key)
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: String, handle: TextureHandle) {
        if self.textures.contains_key(&key) {
            self.textures.insert(key.clone(), handle);
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
            self.order.push_back(key);
            return;
        }

        // Evict oldest if full
        while self.order.len() >= self.max_entries {
            if let Some(oldest) = self.order.pop_front() {
                self.textures.remove(&oldest);
            }
        }

        self.order.push_back(key.clone());
        self.textures.insert(key, handle);
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.textures.clear();
    }
}

/// Central application state.
pub struct AppState {
    pub current_screen: AppScreen,
    pub settings: AppSettings,

    // System info
    pub total_ram_gb: Option<f64>,
    pub ram_mode_detected_notification: Option<String>,

    // Scanner state
    pub scanned_items: Vec<ScannedItem>,
    pub scan_cancel_flag: Arc<AtomicBool>,
    pub scan_events_rx: Option<crossbeam_channel::Receiver<ScanEvent>>,
    pub is_scanning: bool,
    pub scan_base_dir: Option<PathBuf>,

    // Processing state
    pub processing_jobs_tx: Option<crossbeam_channel::Sender<ProcessJob>>,
    pub worker_events_rx: Option<crossbeam_channel::Receiver<WorkerEvent>>,
    pub proc_cancel_flag: Arc<AtomicBool>,
    pub proc_pause_flag: Arc<AtomicBool>,
    pub batch_stats: BatchStats,
    pub processed_photos: Vec<ProcessedPhoto>,
    pub batch_start_time: Option<Instant>,
    pub batch_elapsed_secs: u64,

    // UI Feedback & LRU Cache
    pub thumbnail_cache: ThumbnailCache,
    pub active_preview_source: Option<PhotoSource>,
    pub status_notification: Option<(String, Instant)>,
    pub export_zip_in_progress: bool,

    // Real process memory stats
    pub current_working_set_mb: f64,
    pub peak_working_set_mb: f64,
}

impl AppState {
    pub fn new() -> Self {
        let settings = AppSettings::load();
        let mut total_ram_gb = None;
        let mut ram_notification = None;

        if let Some(bytes) = get_total_physical_ram() {
            let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            total_ram_gb = Some(gb);
            if gb <= 8.5 {
                ram_notification = Some(format!(
                    "Low-memory mode enabled for this computer ({:.1} GB RAM detected).",
                    gb
                ));
            }
        }

        let max_thumbs = settings.memory_mode.max_thumbnails();

        Self {
            current_screen: AppScreen::Start,
            settings,
            total_ram_gb,
            ram_mode_detected_notification: ram_notification,

            scanned_items: Vec::new(),
            scan_cancel_flag: Arc::new(AtomicBool::new(false)),
            scan_events_rx: None,
            is_scanning: false,
            scan_base_dir: None,

            processing_jobs_tx: None,
            worker_events_rx: None,
            proc_cancel_flag: Arc::new(AtomicBool::new(false)),
            proc_pause_flag: Arc::new(AtomicBool::new(false)),
            batch_stats: BatchStats::default(),
            processed_photos: Vec::new(),
            batch_start_time: None,
            batch_elapsed_secs: 0,

            thumbnail_cache: ThumbnailCache::new(max_thumbs),
            active_preview_source: None,
            status_notification: None,
            export_zip_in_progress: false,

            current_working_set_mb: 0.0,
            peak_working_set_mb: 0.0,
        }
    }

    /// Updates memory monitor metrics from native OS APIs.
    pub fn poll_memory_usage(&mut self) {
        if let Some((curr, peak)) = get_process_memory_usage() {
            self.current_working_set_mb = curr as f64 / (1024.0 * 1024.0);
            self.peak_working_set_mb = peak as f64 / (1024.0 * 1024.0);
        }
    }

    /// Sets a temporary status notification toast.
    pub fn show_notification(&mut self, message: impl Into<String>) {
        self.status_notification = Some((message.into(), Instant::now()));
    }

    /// Starts scanning a set of input paths.
    pub fn start_scanning(&mut self, paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return;
        }

        self.scanned_items.clear();
        self.current_screen = AppScreen::Scanning;
        self.is_scanning = true;

        if paths.len() == 1 && paths[0].is_dir() {
            self.scan_base_dir = Some(paths[0].clone());
        } else {
            self.scan_base_dir = None;
        }

        self.scan_cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_flag = Arc::clone(&self.scan_cancel_flag);

        // Bounded channel (capacity 4) to prevent memory ballooning during scanning
        let (tx, rx) = crossbeam_channel::bounded(4);
        self.scan_events_rx = Some(rx);

        std::thread::spawn(move || {
            run_scanner(paths, cancel_flag, tx);
        });
    }

    /// Cancels active scanner.
    pub fn cancel_scan(&mut self) {
        self.scan_cancel_flag.store(true, Ordering::Relaxed);
        self.is_scanning = false;
        self.current_screen = AppScreen::Start;
    }

    /// Polls scan events from background scanner thread.
    pub fn poll_scan_events(&mut self) {
        let events: Vec<ScanEvent> = if let Some(rx) = &self.scan_events_rx {
            let mut collected = Vec::new();
            while let Ok(event) = rx.try_recv() {
                collected.push(event);
            }
            collected
        } else {
            Vec::new()
        };

        for event in events {
            match event {
                ScanEvent::ItemDiscovered(item) => {
                    self.scanned_items.push(item);
                }
                ScanEvent::ScanProgress { .. } => {}
                ScanEvent::ScanFinished { total_discovered, valid_photos, .. } => {
                    self.is_scanning = false;
                    if valid_photos > 0 {
                        self.current_screen = AppScreen::FormatSelect;
                    } else if total_discovered > 0 {
                        self.show_notification("No valid RDR2 photos found in selected inputs.");
                        self.current_screen = AppScreen::Start;
                    } else {
                        self.show_notification("No files discovered.");
                        self.current_screen = AppScreen::Start;
                    }
                }
                ScanEvent::ScanError(err) => {
                    self.show_notification(format!("Scan error: {err}"));
                }
            }
        }
    }

    /// Starts processing batch conversion.
    pub fn start_processing(&mut self) {
        let valid_items: Vec<ScannedItem> = self
            .scanned_items
            .iter()
            .filter(|item| item.is_valid && item.metadata.is_some())
            .cloned()
            .collect();

        if valid_items.is_empty() {
            self.show_notification("No valid photos to convert.");
            return;
        }

        let total = valid_items.len();
        self.current_screen = AppScreen::Processing;
        self.processed_photos.clear();
        self.batch_start_time = Some(Instant::now());
        self.batch_elapsed_secs = 0;

        self.batch_stats = BatchStats {
            total_discovered: self.scanned_items.len(),
            total_valid: total,
            total_invalid: self.scanned_items.len() - total,
            processed_count: 0,
            success_count: 0,
            failed_count: 0,
            input_bytes_total: 0,
            output_bytes_total: 0,
            current_file_name: String::new(),
            is_running: true,
            is_paused: false,
            is_cancelled: false,
        };

        self.proc_cancel_flag = Arc::new(AtomicBool::new(false));
        self.proc_pause_flag = Arc::new(AtomicBool::new(false));

        let cancel_flag = Arc::clone(&self.proc_cancel_flag);
        let pause_flag = Arc::clone(&self.proc_pause_flag);

        // Bounded queue: max 2 jobs in flight
        let (jobs_tx, jobs_rx) = crossbeam_channel::bounded::<ProcessJob>(2);
        let (events_tx, events_rx) = crossbeam_channel::bounded::<WorkerEvent>(2);

        self.processing_jobs_tx = Some(jobs_tx.clone());
        self.worker_events_rx = Some(events_rx);

        let format = self.settings.output_format;
        let base_out = self.settings.output_directory.clone();
        let preserve_names = self.settings.preserve_filenames;
        let preserve_struct = self.settings.preserve_folder_structure;
        let overwrite = self.settings.overwrite_existing;
        let max_ram_mb = self.settings.large_image_threshold_mb;
        let base_in = self.scan_base_dir.clone();

        // 1. Spawn worker thread
        std::thread::spawn(move || {
            run_worker_pool(jobs_rx, events_tx, cancel_flag, pause_flag, total);
        });

        // 2. Spawn feeder thread (feeds bounded queue one-by-one so memory stays bounded)
        let feeder_cancel = Arc::clone(&self.proc_cancel_flag);
        std::thread::spawn(move || {
            for (idx, item) in valid_items.into_iter().enumerate() {
                if feeder_cancel.load(Ordering::Relaxed) {
                    break;
                }
                if let Some(metadata) = item.metadata {
                    let out_path = crate::filesystem::resolve_output_path(
                        &base_out,
                        &item.source,
                        format,
                        preserve_names,
                        preserve_struct,
                        overwrite,
                        idx,
                        base_in.as_deref(),
                    );

                    let job = ProcessJob {
                        index: idx,
                        source: item.source,
                        metadata,
                        output_path: out_path,
                        format,
                        max_memory_mb: max_ram_mb,
                    };

                    if jobs_tx.send(job).is_err() {
                        break;
                    }
                }
            }
        });
    }

    /// Retries failed items from previous batch.
    pub fn retry_failed_items(&mut self) {
        let failed_sources: Vec<PhotoSource> = self
            .processed_photos
            .iter()
            .filter(|p| matches!(p.status, ProcessStatus::Failed(_)))
            .map(|p| p.source.clone())
            .collect();

        if failed_sources.is_empty() {
            return;
        }

        self.scanned_items.retain(|item| failed_sources.contains(&item.source));
        self.start_processing();
    }

    /// Toggles pause/resume state of the batch.
    pub fn toggle_pause(&mut self) {
        let current = self.proc_pause_flag.load(Ordering::Relaxed);
        let next = !current;
        self.proc_pause_flag.store(next, Ordering::Relaxed);
        self.batch_stats.is_paused = next;
    }

    /// Cancels active processing safely.
    pub fn cancel_processing(&mut self) {
        self.proc_cancel_flag.store(true, Ordering::Relaxed);
        self.batch_stats.is_cancelled = true;
        self.batch_stats.is_running = false;
        self.current_screen = AppScreen::Result;
    }

    /// Polls worker events from background processor.
    pub fn poll_worker_events(&mut self) {
        let events: Vec<WorkerEvent> = if let Some(rx) = &self.worker_events_rx {
            let mut collected = Vec::new();
            while let Ok(event) = rx.try_recv() {
                collected.push(event);
            }
            collected
        } else {
            Vec::new()
        };

        for event in events {
            match event {
                WorkerEvent::JobStarted { source_name, .. } => {
                    self.batch_stats.current_file_name = source_name;
                }
                WorkerEvent::JobFinished(processed) => {
                    self.processed_photos.push(processed);
                }
                WorkerEvent::BatchProgress {
                    completed,
                    success_count,
                    failed_count,
                    input_bytes,
                    output_bytes,
                    current_file,
                    ..
                } => {
                    self.batch_stats.processed_count = completed;
                    self.batch_stats.success_count = success_count;
                    self.batch_stats.failed_count = failed_count;
                    self.batch_stats.input_bytes_total = input_bytes;
                    self.batch_stats.output_bytes_total = output_bytes;
                    self.batch_stats.current_file_name = current_file;
                    if let Some(start) = self.batch_start_time {
                        self.batch_elapsed_secs = start.elapsed().as_secs();
                    }
                }
                WorkerEvent::BatchFinished {
                    success_count,
                    failed_count,
                    input_bytes,
                    output_bytes,
                    cancelled,
                    ..
                } => {
                    self.batch_stats.is_running = false;
                    self.batch_stats.is_cancelled = cancelled;
                    self.batch_stats.success_count = success_count;
                    self.batch_stats.failed_count = failed_count;
                    self.batch_stats.input_bytes_total = input_bytes;
                    self.batch_stats.output_bytes_total = output_bytes;
                    self.current_screen = AppScreen::Result;
                }
            }
        }
    }

    /// Resets state to begin a fresh batch.
    pub fn reset_to_start(&mut self) {
        self.scanned_items.clear();
        self.processed_photos.clear();
        self.thumbnail_cache.clear();
        self.active_preview_source = None;
        self.batch_stats = BatchStats::default();
        self.current_screen = AppScreen::Start;
    }
}
