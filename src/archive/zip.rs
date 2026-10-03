use std::fs::File;
use std::io::{self, BufReader, BufWriter, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};
use crate::extractor::scan_for_jpeg;
use crate::models::{PhotoSource, ScannedItem};

#[derive(Debug, thiserror::Error)]
pub enum ZipError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Zip archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Security violation: ZIP path traversal detected for entry '{0}'")]
    PathTraversal(String),
}

/// Sanitizes and validates a ZIP entry name to prevent path traversal attacks.
pub fn validate_zip_entry_name(name: &str) -> Result<String, ZipError> {
    if name.contains("..") || name.starts_with('/') || name.starts_with('\\') || name.contains(':') {
        return Err(ZipError::PathTraversal(name.to_string()));
    }

    let path = Path::new(name);
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir | std::path::Component::RootDir | std::path::Component::Prefix(_)) {
            return Err(ZipError::PathTraversal(name.to_string()));
        }
    }

    let clean_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "photo".to_string());

    Ok(clean_name)
}

/// Scans a ZIP archive file for embedded RDR2 photos entry by entry.
/// Streams entries one at a time and frees memory after each entry.
pub fn scan_zip_archive(zip_path: &Path) -> Result<Vec<ScannedItem>, ZipError> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;
    let mut results = Vec::new();

    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };

        if entry.is_dir() {
            continue;
        }

        let raw_name = entry.name().to_string();
        let safe_name = match validate_zip_entry_name(&raw_name) {
            Ok(name) => name,
            Err(e) => {
                results.push(ScannedItem {
                    source: PhotoSource::ZipEntry {
                        zip_path: zip_path.to_path_buf(),
                        entry_name: raw_name,
                        entry_index: i,
                    },
                    file_size: entry.size(),
                    is_valid: false,
                    metadata: None,
                    error_message: Some(e.to_string()),
                });
                continue;
            }
        };

        let entry_size = entry.size();
        if entry_size < 128 {
            results.push(ScannedItem {
                source: PhotoSource::ZipEntry {
                    zip_path: zip_path.to_path_buf(),
                    entry_name: safe_name,
                    entry_index: i,
                },
                file_size: entry_size,
                is_valid: false,
                metadata: None,
                error_message: Some("Entry is too small to contain a JPEG".to_string()),
            });
            continue;
        }

        // Read entry data into a memory cursor for inspection (only one entry at a time)
        let mut entry_bytes = Vec::with_capacity(entry_size.min(16 * 1024 * 1024) as usize);
        if let Err(e) = entry.read_to_end(&mut entry_bytes) {
            results.push(ScannedItem {
                source: PhotoSource::ZipEntry {
                    zip_path: zip_path.to_path_buf(),
                    entry_name: safe_name,
                    entry_index: i,
                },
                file_size: entry_size,
                is_valid: false,
                metadata: None,
                error_message: Some(format!("Failed to read ZIP entry: {e}")),
            });
            continue;
        }

        let mut cursor = Cursor::new(&entry_bytes);
        match scan_for_jpeg(&mut cursor, entry_bytes.len() as u64) {
            Ok(Some(meta)) => {
                results.push(ScannedItem {
                    source: PhotoSource::ZipEntry {
                        zip_path: zip_path.to_path_buf(),
                        entry_name: safe_name,
                        entry_index: i,
                    },
                    file_size: entry_size,
                    is_valid: true,
                    metadata: Some(meta),
                    error_message: None,
                });
            }
            Ok(None) => {
                results.push(ScannedItem {
                    source: PhotoSource::ZipEntry {
                        zip_path: zip_path.to_path_buf(),
                        entry_name: safe_name,
                        entry_index: i,
                    },
                    file_size: entry_size,
                    is_valid: false,
                    metadata: None,
                    error_message: Some("No embedded JPEG found in entry".to_string()),
                });
            }
            Err(e) => {
                results.push(ScannedItem {
                    source: PhotoSource::ZipEntry {
                        zip_path: zip_path.to_path_buf(),
                        entry_name: safe_name,
                        entry_index: i,
                    },
                    file_size: entry_size,
                    is_valid: false,
                    metadata: None,
                    error_message: Some(format!("Scanner error: {e}")),
                });
            }
        }
        drop(entry_bytes); // Free memory immediately
    }

    Ok(results)
}

/// Reads a specific entry from a ZIP archive as a seekable stream for conversion.
pub fn read_zip_entry_to_cursor(
    zip_path: &Path,
    entry_index: usize,
) -> Result<Cursor<Vec<u8>>, ZipError> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(BufReader::new(file))?;
    let mut entry = archive.by_index(entry_index)?;

    validate_zip_entry_name(entry.name())?;

    let mut data = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut data)?;
    Ok(Cursor::new(data))
}

/// Incremental streaming ZIP exporter that writes converted photos directly to disk
/// without accumulating images in RAM.
pub struct StreamingZipExporter {
    writer: ZipWriter<BufWriter<File>>,
    output_path: PathBuf,
    files_written: usize,
}

impl StreamingZipExporter {
    pub fn new(output_zip_path: &Path) -> Result<Self, ZipError> {
        if let Some(parent) = output_zip_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(output_zip_path)?;
        let buf_writer = BufWriter::with_capacity(128 * 1024, file);
        let zip_writer = ZipWriter::new(buf_writer);

        Ok(Self {
            writer: zip_writer,
            output_path: output_zip_path.to_path_buf(),
            files_written: 0,
        })
    }

    /// Adds a file from disk into the ZIP archive using chunked streaming.
    pub fn add_file_from_disk(&mut self, entry_name: &str, file_path: &Path) -> Result<(), ZipError> {
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        self.writer.start_file(entry_name, options)?;

        let file = File::open(file_path)?;
        let mut reader = BufReader::with_capacity(64 * 1024, file);
        let mut buffer = [0u8; 64 * 1024];

        loop {
            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            self.writer.write_all(&buffer[..n])?;
        }

        self.files_written += 1;
        Ok(())
    }

    /// Finalizes the ZIP archive and flushes to disk.
    pub fn finish(self) -> Result<u64, ZipError> {
        self.writer.finish()?;
        let meta = std::fs::metadata(&self.output_path)?;
        Ok(meta.len())
    }
}
