use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Seek};
use std::path::Path;
use crate::extractor::stream_extract_jpeg;
use crate::models::PhotoMetadata;

/// Extracts the embedded JPEG from `source_reader` directly to `dest_path` without re-encoding.
/// Uses buffered I/O with 64 KB chunks to maintain low, constant O(1) RAM usage.
pub fn extract_jpeg_to_file<R: Read + Seek>(
    source_reader: &mut R,
    dest_path: &Path,
    metadata: &PhotoMetadata,
) -> io::Result<u64> {
    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(dest_path)?;
    let mut writer = BufWriter::with_capacity(64 * 1024, file);

    let written = stream_extract_jpeg(
        source_reader,
        &mut writer,
        metadata.jpeg_offset,
        metadata.jpeg_len,
    )?;

    Ok(written)
}

/// Extracts the embedded JPEG directly from a file on disk to a destination file on disk.
pub fn extract_jpeg_from_file_to_file(
    source_path: &Path,
    dest_path: &Path,
    metadata: &PhotoMetadata,
) -> io::Result<u64> {
    let file = File::open(source_path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    extract_jpeg_to_file(&mut reader, dest_path, metadata)
}
