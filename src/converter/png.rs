use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Seek};
use std::path::Path;
use image::{ImageFormat, DynamicImage};
use crate::extractor::read_jpeg_bytes;
use crate::models::PhotoMetadata;

#[derive(Debug, thiserror::Error)]
pub enum PngConversionError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Image decode error: {0}")]
    ImageDecode(#[from] image::ImageError),
    #[error("Image dimensions ({0} \u{00D7} {1}) exceed safe memory limit ({2:.1} MB required)")]
    MemoryLimitExceeded(u32, u32, f32),
}

/// Converts the embedded JPEG inside `source_reader` into a lossless PNG file at `dest_path`.
/// Decodes only this single image and drops all pixel buffers immediately upon completion.
pub fn convert_to_png<R: Read + Seek>(
    source_reader: &mut R,
    dest_path: &Path,
    metadata: &PhotoMetadata,
    max_memory_mb: u64,
) -> Result<u64, PngConversionError> {
    // Memory safety check
    let est_mb = metadata.estimated_decode_ram_mb();
    if max_memory_mb > 0 && est_mb > max_memory_mb as f32 {
        return Err(PngConversionError::MemoryLimitExceeded(
            metadata.width,
            metadata.height,
            est_mb,
        ));
    }

    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 1. Read only the JPEG bytes for this one photo
    let jpeg_bytes = read_jpeg_bytes(source_reader, metadata.jpeg_offset, metadata.jpeg_len)?;

    // 2. Decode the single JPEG into memory
    let img: DynamicImage = image::load_from_memory_with_format(&jpeg_bytes, ImageFormat::Jpeg)?;
    drop(jpeg_bytes); // Free raw JPEG buffer immediately

    // 3. Encode directly to PNG file using buffered writer
    let file = File::create(dest_path)?;
    let mut writer = BufWriter::with_capacity(64 * 1024, file);
    img.write_to(&mut writer, ImageFormat::Png)?;

    // 4. Drop decoded image buffer immediately
    drop(img);

    // Get final output size
    let metadata_out = std::fs::metadata(dest_path)?;
    Ok(metadata_out.len())
}

/// Converts from a local file on disk to a PNG file on disk.
pub fn convert_file_to_png(
    source_path: &Path,
    dest_path: &Path,
    metadata: &PhotoMetadata,
    max_memory_mb: u64,
) -> Result<u64, PngConversionError> {
    let file = File::open(source_path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    convert_to_png(&mut reader, dest_path, metadata, max_memory_mb)
}

/// Generates a small thumbnail (max 320px) from the embedded JPEG on demand.
/// Drops all full-resolution buffers immediately, returning only the small thumbnail.
pub fn generate_thumbnail<R: Read + Seek>(
    source_reader: &mut R,
    metadata: &PhotoMetadata,
    max_dim: u32,
) -> Result<image::RgbaImage, PngConversionError> {
    let jpeg_bytes = read_jpeg_bytes(source_reader, metadata.jpeg_offset, metadata.jpeg_len)?;
    let img = image::load_from_memory_with_format(&jpeg_bytes, ImageFormat::Jpeg)?;
    drop(jpeg_bytes);

    let thumbnail = img.thumbnail(max_dim, max_dim);
    drop(img); // Free full resolution image immediately!

    Ok(thumbnail.into_rgba8())
}
