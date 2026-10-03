use std::io::Cursor;
use blackwater::extractor::{scan_for_jpeg, stream_extract_jpeg};
use blackwater::converter::extract_jpeg_to_file;

/// Creates a synthetic valid minimal JPEG byte buffer with given width and height.
pub fn create_test_jpeg_bytes(width: u16, height: u16) -> Vec<u8> {
    let mut jpeg = Vec::new();
    // SOI
    jpeg.extend_from_slice(&[0xFF, 0xD8]);
    
    // APP0 (JFIF)
    jpeg.extend_from_slice(&[
        0xFF, 0xE0, 0x00, 0x10,
        0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00, 0x48, 0x00, 0x48, 0x00, 0x00,
    ]);

    // DQT (Quantization table)
    jpeg.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x43, 0x00]);
    jpeg.extend_from_slice(&[16u8; 64]);

    // SOF0 (Start of Frame: Baseline DCT)
    let w_bytes = width.to_be_bytes();
    let h_bytes = height.to_be_bytes();
    jpeg.extend_from_slice(&[
        0xFF, 0xC0, 0x00, 0x0B,
        0x08, // precision 8-bit
        h_bytes[0], h_bytes[1],
        w_bytes[0], w_bytes[1],
        0x01, // 1 component (grayscale)
        0x01, 0x11, 0x00,
    ]);

    // DHT (Huffman table)
    jpeg.extend_from_slice(&[
        0xFF, 0xC4, 0x00, 0x1F, 0x00,
        0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    ]);

    // SOS (Start of Scan)
    jpeg.extend_from_slice(&[
        0xFF, 0xDA, 0x00, 0x08,
        0x01, 0x01, 0x00, 0x00, 0x3F, 0x00,
    ]);

    // Minimal Entropy payload
    jpeg.extend_from_slice(&[0x00, 0x7F, 0xFF, 0x00, 0x55]);

    // EOI
    jpeg.extend_from_slice(&[0xFF, 0xD9]);

    jpeg
}

/// Embeds a JPEG inside a synthetic proprietary RDR2 container format.
pub fn create_rdr2_container(prefix_len: usize, width: u16, height: u16, suffix_len: usize) -> (Vec<u8>, Vec<u8>) {
    let raw_jpeg = create_test_jpeg_bytes(width, height);
    let mut container = vec![0xCC; prefix_len]; // Proprietary header bytes
    let _start_offset = container.len();
    container.extend_from_slice(&raw_jpeg);
    container.extend_from_slice(&vec![0xAA; suffix_len]); // Trailing metadata
    (container, raw_jpeg)
}

#[test]
fn test_jpeg_marker_detection_and_dimensions() {
    let (container, raw_jpeg) = create_rdr2_container(350, 1920, 1080, 128);
    let mut cursor = Cursor::new(&container);

    let result = scan_for_jpeg(&mut cursor, container.len() as u64)
        .expect("Scan should succeed")
        .expect("Should find embedded JPEG");

    assert_eq!(result.jpeg_offset, 350);
    assert_eq!(result.jpeg_len, raw_jpeg.len() as u64);
    assert_eq!(result.width, 1920);
    assert_eq!(result.height, 1080);
}

#[test]
fn test_byte_exact_jpeg_extraction() {
    let (container, expected_jpeg) = create_rdr2_container(512, 800, 600, 200);
    let mut cursor = Cursor::new(&container);

    let meta = scan_for_jpeg(&mut cursor, container.len() as u64)
        .unwrap()
        .unwrap();

    let temp_dir = tempfile::tempdir().unwrap();
    let out_file = temp_dir.path().join("extracted.jpg");

    let written = extract_jpeg_to_file(&mut cursor, &out_file, &meta).unwrap();
    assert_eq!(written, expected_jpeg.len() as u64);

    let extracted_bytes = std::fs::read(&out_file).unwrap();
    assert_eq!(extracted_bytes, expected_jpeg, "Extracted JPEG must match source byte-for-byte!");
}

#[test]
fn test_invalid_file_handling() {
    // 1. Empty buffer
    let mut empty_cursor = Cursor::new(Vec::new());
    let res = scan_for_jpeg(&mut empty_cursor, 0).unwrap();
    assert!(res.is_none());

    // 2. Random binary garbage
    let random_data = vec![0x42; 10000];
    let mut garbage_cursor = Cursor::new(&random_data);
    let res = scan_for_jpeg(&mut garbage_cursor, random_data.len() as u64).unwrap();
    assert!(res.is_none());

    // 3. Truncated SOI without EOI
    let broken_soi = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    let mut broken_cursor = Cursor::new(&broken_soi);
    let res = scan_for_jpeg(&mut broken_cursor, broken_soi.len() as u64).unwrap();
    assert!(res.is_none());
}

#[test]
fn test_streaming_chunk_extraction() {
    let (container, expected_jpeg) = create_rdr2_container(1024 * 128, 640, 480, 1024 * 64);
    let mut cursor = Cursor::new(&container);
    let mut out_buffer = Vec::new();

    let written = stream_extract_jpeg(
        &mut cursor,
        &mut out_buffer,
        1024 * 128,
        expected_jpeg.len() as u64,
    ).unwrap();

    assert_eq!(written, expected_jpeg.len() as u64);
    assert_eq!(out_buffer, expected_jpeg);
}
