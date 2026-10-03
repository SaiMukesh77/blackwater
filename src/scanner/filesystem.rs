use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;
use crate::archive::scan_zip_archive;
use crate::extractor::scan_for_jpeg;
use crate::models::{PhotoSource, ScannedItem};

#[derive(Debug, Clone)]
pub enum ScanEvent {
    ItemDiscovered(ScannedItem),
    ScanProgress {
        total_discovered: usize,
        valid_photos: usize,
        invalid_files: usize,
    },
    ScanFinished {
        total_discovered: usize,
        valid_photos: usize,
        invalid_files: usize,
    },
    ScanError(String),
}

/// Scans a single local file on disk without relying on extension.
pub fn scan_single_file(path: &Path) -> ScannedItem {
    let metadata_res = std::fs::metadata(path);
    let file_size = match metadata_res {
        Ok(m) => m.len(),
        Err(e) => {
            return ScannedItem {
                source: PhotoSource::LocalFile(path.to_path_buf()),
                file_size: 0,
                is_valid: false,
                metadata: None,
                error_message: Some(format!("Cannot read file metadata: {e}")),
            };
        }
    };

    if file_size < 128 {
        return ScannedItem {
            source: PhotoSource::LocalFile(path.to_path_buf()),
            file_size,
            is_valid: false,
            metadata: None,
            error_message: Some("File is too small to contain a valid JPEG".to_string()),
        };
    }

    // Check if it's a ZIP archive
    if is_zip_file(path) {
        // Zip archives are handled via scan_zip_archive
        return ScannedItem {
            source: PhotoSource::LocalFile(path.to_path_buf()),
            file_size,
            is_valid: false,
            metadata: None,
            error_message: Some("ZIP archive should be expanded".to_string()),
        };
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            return ScannedItem {
                source: PhotoSource::LocalFile(path.to_path_buf()),
                file_size,
                is_valid: false,
                metadata: None,
                error_message: Some(format!("Failed to open file: {e}")),
            };
        }
    };

    let mut reader = BufReader::with_capacity(64 * 1024, file);
    match scan_for_jpeg(&mut reader, file_size) {
        Ok(Some(meta)) => ScannedItem {
            source: PhotoSource::LocalFile(path.to_path_buf()),
            file_size,
            is_valid: true,
            metadata: Some(meta),
            error_message: None,
        },
        Ok(None) => ScannedItem {
            source: PhotoSource::LocalFile(path.to_path_buf()),
            file_size,
            is_valid: false,
            metadata: None,
            error_message: Some("No embedded JPEG found".to_string()),
        },
        Err(e) => ScannedItem {
            source: PhotoSource::LocalFile(path.to_path_buf()),
            file_size,
            is_valid: false,
            metadata: None,
            error_message: Some(format!("Error parsing file: {e}")),
        },
    }
}

/// Checks if a file begins with standard PK ZIP magic bytes (0x50, 0x4B, 0x03, 0x04)
pub fn is_zip_file(path: &Path) -> bool {
    use std::io::Read;
    if let Ok(mut file) = File::open(path) {
        let mut magic = [0u8; 4];
        if file.read_exact(&mut magic).is_ok() {
            return magic == [0x50, 0x4B, 0x03, 0x04] || magic == [0x50, 0x4B, 0x05, 0x06];
        }
    }
    false
}

/// Runs a bounded directory/file scan in a background task, sending events over a bounded channel.
pub fn run_scanner(
    inputs: Vec<PathBuf>,
    cancel_flag: Arc<AtomicBool>,
    sender: crossbeam_channel::Sender<ScanEvent>,
) {
    let mut total_discovered = 0;
    let mut valid_photos = 0;
    let mut invalid_files = 0;

    for input_path in inputs {
        if cancel_flag.load(Ordering::Relaxed) {
            break;
        }

        if input_path.is_file() {
            if is_zip_file(&input_path) {
                // Scan ZIP archive
                match scan_zip_archive(&input_path) {
                    Ok(items) => {
                        for item in items {
                            if cancel_flag.load(Ordering::Relaxed) {
                                break;
                            }
                            total_discovered += 1;
                            if item.is_valid {
                                valid_photos += 1;
                            } else {
                                invalid_files += 1;
                            }
                            let _ = sender.send(ScanEvent::ItemDiscovered(item));
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(ScanEvent::ScanError(format!("ZIP Error {}: {e}", input_path.display())));
                    }
                }
            } else {
                let item = scan_single_file(&input_path);
                total_discovered += 1;
                if item.is_valid {
                    valid_photos += 1;
                } else {
                    invalid_files += 1;
                }
                let _ = sender.send(ScanEvent::ItemDiscovered(item));
            }
        } else if input_path.is_dir() {
            // Recursive directory traversal
            for entry in WalkDir::new(&input_path).into_iter().filter_map(|e| e.ok()) {
                if cancel_flag.load(Ordering::Relaxed) {
                    break;
                }
                if entry.file_type().is_file() {
                    let path = entry.path();
                    if is_zip_file(path) {
                        if let Ok(items) = scan_zip_archive(path) {
                            for item in items {
                                if cancel_flag.load(Ordering::Relaxed) {
                                    break;
                                }
                                total_discovered += 1;
                                if item.is_valid {
                                    valid_photos += 1;
                                } else {
                                    invalid_files += 1;
                                }
                                let _ = sender.send(ScanEvent::ItemDiscovered(item));
                            }
                        }
                    } else {
                        let item = scan_single_file(path);
                        total_discovered += 1;
                        if item.is_valid {
                            valid_photos += 1;
                        } else {
                            invalid_files += 1;
                        }
                        let _ = sender.send(ScanEvent::ItemDiscovered(item));
                    }
                }

                if total_discovered % 10 == 0 {
                    let _ = sender.send(ScanEvent::ScanProgress {
                        total_discovered,
                        valid_photos,
                        invalid_files,
                    });
                }
            }
        }
    }

    let _ = sender.send(ScanEvent::ScanFinished {
        total_discovered,
        valid_photos,
        invalid_files,
    });
}
