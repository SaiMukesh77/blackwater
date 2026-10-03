use std::path::PathBuf;
use crate::models::{OutputFormat, PhotoMetadata, PhotoSource, ProcessedPhoto};

/// Commands sent from the UI to the background processing engine.
#[derive(Debug, Clone)]
pub struct ProcessJob {
    pub index: usize,
    pub source: PhotoSource,
    pub metadata: PhotoMetadata,
    pub output_path: PathBuf,
    pub format: OutputFormat,
    pub max_memory_mb: u64,
}

/// Events sent from the worker thread(s) back to the UI.
#[derive(Debug, Clone)]
pub enum WorkerEvent {
    JobStarted {
        index: usize,
        source_name: String,
    },
    JobFinished(ProcessedPhoto),
    BatchProgress {
        completed: usize,
        total: usize,
        success_count: usize,
        failed_count: usize,
        input_bytes: u64,
        output_bytes: u64,
        current_file: String,
    },
    BatchFinished {
        total: usize,
        success_count: usize,
        failed_count: usize,
        input_bytes: u64,
        output_bytes: u64,
        cancelled: bool,
    },
}
