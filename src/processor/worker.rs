use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use crate::archive::read_zip_entry_to_cursor;
use crate::converter::{convert_file_to_png, convert_to_png, extract_jpeg_from_file_to_file, extract_jpeg_to_file};
use crate::models::{OutputFormat, PhotoSource, ProcessStatus, ProcessedPhoto};
use crate::processor::queue::{ProcessJob, WorkerEvent};

/// Runs the bounded worker pool processing jobs one-by-one with low, constant RAM usage.
pub fn run_worker_pool(
    jobs_receiver: crossbeam_channel::Receiver<ProcessJob>,
    events_sender: crossbeam_channel::Sender<WorkerEvent>,
    cancel_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
    total_jobs: usize,
) {
    let mut completed = 0;
    let mut success_count = 0;
    let mut failed_count = 0;
    let mut total_input_bytes = 0u64;
    let mut total_output_bytes = 0u64;

    while let Ok(job) = jobs_receiver.recv() {
        // 1. Check for cancellation
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }

        // 2. Check for pause state
        while pause_flag.load(Ordering::Relaxed) {
            if cancel_flag.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }

        // 3. Notify UI that job is starting
        let _ = events_sender.send(WorkerEvent::JobStarted {
            index: job.index,
            source_name: job.source.display_name(),
        });

        let start_time = Instant::now();
        let mut job_success = false;
        let mut error_msg: Option<String> = None;

        // 4. Process single image
        match process_single_job(&job) {
            Ok(bytes) => {
                job_success = true;
                success_count += 1;
                total_output_bytes += bytes;
                total_input_bytes += job.metadata.source_size_bytes;
            }
            Err(e) => {
                failed_count += 1;
                error_msg = Some(e);
                // Safe cleanup: delete partial / corrupted output file if left over
                cleanup_partial_file(&job.output_path);
            }
        }

        completed += 1;
        let elapsed = start_time.elapsed().as_millis();

        let status = if job_success {
            ProcessStatus::Success
        } else {
            ProcessStatus::Failed(error_msg.unwrap_or_else(|| "Unknown processing error".to_string()))
        };

        let processed = ProcessedPhoto {
            index: job.index,
            source: job.source.clone(),
            output_path: if job_success { Some(job.output_path.clone()) } else { None },
            status,
            metadata: Some(job.metadata.clone()),
            duration_ms: elapsed,
        };

        let _ = events_sender.send(WorkerEvent::JobFinished(processed));

        let _ = events_sender.send(WorkerEvent::BatchProgress {
            completed,
            total: total_jobs,
            success_count,
            failed_count,
            input_bytes: total_input_bytes,
            output_bytes: total_output_bytes,
            current_file: job.source.display_name(),
        });
    }

    let is_cancelled = cancel_flag.load(Ordering::Relaxed);
    let _ = events_sender.send(WorkerEvent::BatchFinished {
        total: total_jobs,
        success_count,
        failed_count,
        input_bytes: total_input_bytes,
        output_bytes: total_output_bytes,
        cancelled: is_cancelled,
    });
}

fn process_single_job(job: &ProcessJob) -> Result<u64, String> {
    match (&job.source, job.format) {
        (PhotoSource::LocalFile(local_path), OutputFormat::Jpeg) => {
            extract_jpeg_from_file_to_file(local_path, &job.output_path, &job.metadata)
                .map_err(|e| format!("Failed to extract JPEG: {e}"))
        }
        (PhotoSource::LocalFile(local_path), OutputFormat::Png) => {
            convert_file_to_png(local_path, &job.output_path, &job.metadata, job.max_memory_mb)
                .map_err(|e| format!("Failed to convert to PNG: {e}"))
        }
        (PhotoSource::ZipEntry { zip_path, entry_index, .. }, OutputFormat::Jpeg) => {
            let mut cursor = read_zip_entry_to_cursor(zip_path, *entry_index)
                .map_err(|e| format!("ZIP entry read failed: {e}"))?;
            extract_jpeg_to_file(&mut cursor, &job.output_path, &job.metadata)
                .map_err(|e| format!("Failed to extract JPEG from ZIP: {e}"))
        }
        (PhotoSource::ZipEntry { zip_path, entry_index, .. }, OutputFormat::Png) => {
            let mut cursor = read_zip_entry_to_cursor(zip_path, *entry_index)
                .map_err(|e| format!("ZIP entry read failed: {e}"))?;
            convert_to_png(&mut cursor, &job.output_path, &job.metadata, job.max_memory_mb)
                .map_err(|e| format!("Failed to convert ZIP entry to PNG: {e}"))
        }
    }
}

fn cleanup_partial_file(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}
