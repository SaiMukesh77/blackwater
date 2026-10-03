use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Identifies the source of a photograph to be processed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PhotoSource {
    /// A standalone file on the filesystem (with or without extension)
    LocalFile(PathBuf),
    /// An entry within a ZIP archive
    ZipEntry {
        zip_path: PathBuf,
        entry_name: String,
        entry_index: usize,
    },
}

impl PhotoSource {
    pub fn display_name(&self) -> String {
        match self {
            PhotoSource::LocalFile(path) => {
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string())
            }
            PhotoSource::ZipEntry { entry_name, .. } => {
                entry_name
                    .split('/')
                    .last()
                    .unwrap_or(entry_name.as_str())
                    .to_string()
            }
        }
    }

    pub fn full_path_str(&self) -> String {
        match self {
            PhotoSource::LocalFile(path) => path.to_string_lossy().to_string(),
            PhotoSource::ZipEntry { zip_path, entry_name, .. } => {
                format!("{} / {}", zip_path.file_name().unwrap_or_default().to_string_lossy(), entry_name)
            }
        }
    }
}

/// Target output format for extracted photos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OutputFormat {
    /// Direct byte extraction of the original JPEG without re-encoding
    #[default]
    Jpeg,
    /// Conversion from extracted JPEG to lossless PNG
    Png,
}

impl OutputFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Jpeg => "jpg",
            OutputFormat::Png => "png",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            OutputFormat::Jpeg => "JPEG",
            OutputFormat::Png => "PNG",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            OutputFormat::Jpeg => "Extract original embedded JPEG without re-encoding.",
            OutputFormat::Png => "Convert the extracted JPEG to lossless PNG.",
        }
    }
}

/// Memory operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MemoryMode {
    /// 1 worker, strict memory bounding, max 20 thumbnails
    #[default]
    LowMemory,
    /// Max 2 workers, max 40 thumbnails
    Balanced,
}

impl MemoryMode {
    pub fn max_workers(&self) -> usize {
        match self {
            MemoryMode::LowMemory => 1,
            MemoryMode::Balanced => 2,
        }
    }

    pub fn max_thumbnails(&self) -> usize {
        match self {
            MemoryMode::LowMemory => 20,
            MemoryMode::Balanced => 40,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            MemoryMode::LowMemory => "LOW MEMORY",
            MemoryMode::Balanced => "BALANCED",
        }
    }
}

/// Image metadata parsed directly from JPEG markers without full pixel decoding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhotoMetadata {
    pub width: u32,
    pub height: u32,
    pub jpeg_offset: u64,
    pub jpeg_len: u64,
    pub source_size_bytes: u64,
    pub output_size_bytes: u64,
}

impl PhotoMetadata {
    /// Returns the estimated RAM in megabytes needed to decode this image to raw RGBA pixels.
    pub fn estimated_decode_ram_mb(&self) -> f32 {
        (self.width as f64 * self.height as f64 * 4.0 / (1024.0 * 1024.0)) as f32
    }

    pub fn resolution_str(&self) -> String {
        format!("{} \u{00D7} {}", self.width, self.height)
    }
}

/// Result of scanning an input file or archive entry.
#[derive(Debug, Clone)]
pub struct ScannedItem {
    pub source: PhotoSource,
    pub file_size: u64,
    pub is_valid: bool,
    pub metadata: Option<PhotoMetadata>,
    pub error_message: Option<String>,
}

/// Result of converting an individual photo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessStatus {
    Pending,
    Processing,
    Success,
    Failed(String),
    Skipped(String),
}

/// Information about a processed photo in the result list.
#[derive(Debug, Clone)]
pub struct ProcessedPhoto {
    pub index: usize,
    pub source: PhotoSource,
    pub output_path: Option<PathBuf>,
    pub status: ProcessStatus,
    pub metadata: Option<PhotoMetadata>,
    pub duration_ms: u128,
}

/// Summary statistics of a batch run.
#[derive(Debug, Clone, Default)]
pub struct BatchStats {
    pub total_discovered: usize,
    pub total_valid: usize,
    pub total_invalid: usize,
    pub processed_count: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub input_bytes_total: u64,
    pub output_bytes_total: u64,
    pub current_file_name: String,
    pub is_running: bool,
    pub is_paused: bool,
    pub is_cancelled: bool,
}
