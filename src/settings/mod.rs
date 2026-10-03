use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::models::{MemoryMode, OutputFormat};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub output_format: OutputFormat,
    pub output_directory: PathBuf,
    pub worker_count: usize,
    pub memory_mode: MemoryMode,
    pub overwrite_existing: bool,
    pub preserve_filenames: bool,
    pub preserve_folder_structure: bool,
    pub generate_thumbnails: bool,
    pub large_image_threshold_mb: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        let (detected_mode, default_workers) = detect_system_ram_recommendations();
        Self {
            output_format: OutputFormat::Jpeg,
            output_directory: default_output_directory(),
            worker_count: default_workers,
            memory_mode: detected_mode,
            overwrite_existing: false,
            preserve_filenames: true,
            preserve_folder_structure: false,
            generate_thumbnails: true,
            large_image_threshold_mb: 256,
        }
    }
}

pub fn default_output_directory() -> PathBuf {
    if let Some(docs) = dirs::document_dir() {
        docs.join("Blackwater")
    } else {
        PathBuf::from("./Blackwater_Output")
    }
}

/// Detects total installed physical RAM and returns (recommended_mode, recommended_workers).
pub fn detect_system_ram_recommendations() -> (MemoryMode, usize) {
    if let Some(total_bytes) = get_total_physical_ram() {
        let gb = total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        if gb <= 8.5 {
            // 8 GB RAM or less: Strict LOW MEMORY mode, 1 worker
            (MemoryMode::LowMemory, 1)
        } else {
            (MemoryMode::LowMemory, 1) // Default to low memory for rock-solid low RAM
        }
    } else {
        (MemoryMode::LowMemory, 1)
    }
}

/// Returns total physical RAM in bytes if queryable via native OS APIs.
pub fn get_total_physical_ram() -> Option<u64> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        use std::mem::zeroed;
        unsafe {
            let mut status: MEMORYSTATUSEX = zeroed();
            status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut status) != 0 {
                return Some(status.ullTotalPhys);
            }
        }
    }
    None
}

/// Returns current process working set (RAM used) in bytes if queryable.
pub fn get_process_memory_usage() -> Option<(u64, u64)> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        use std::mem::zeroed;
        unsafe {
            let handle = GetCurrentProcess();
            let mut counters: PROCESS_MEMORY_COUNTERS = zeroed();
            let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
            counters.cb = size;
            if GetProcessMemoryInfo(handle, &mut counters, size) != 0 {
                return Some((counters.WorkingSetSize as u64, counters.PeakWorkingSetSize as u64));
            }
        }
    }
    None
}

impl AppSettings {
    fn config_file_path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("Blackwater").join("config.json"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_file_path() {
            if path.exists() {
                if let Ok(file) = std::fs::File::open(&path) {
                    if let Ok(settings) = serde_json::from_reader(file) {
                        return settings;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(path) = Self::config_file_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Ok(file) = std::fs::File::create(&path) {
                let _ = serde_json::to_writer_pretty(file, self);
            }
        }
    }
}
