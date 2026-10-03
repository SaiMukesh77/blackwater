use std::fs::File;
use std::io::Write;
use blackwater::archive::{scan_zip_archive, validate_zip_entry_name, StreamingZipExporter};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[test]
fn test_zip_path_traversal_prevention() {
    // Malicious traversal attempts
    assert!(validate_zip_entry_name("../../evil.txt").is_err());
    assert!(validate_zip_entry_name("..\\..\\malicious.dll").is_err());
    assert!(validate_zip_entry_name("/etc/passwd").is_err());
    assert!(validate_zip_entry_name("\\Windows\\System32").is_err());
    assert!(validate_zip_entry_name("C:\\Users\\Admin").is_err());
    assert!(validate_zip_entry_name("folder/../../../secret").is_err());

    // Safe entry names
    assert!(validate_zip_entry_name("photo1.jpg").is_ok());
    assert!(validate_zip_entry_name("rdr2_photos/PRDR3852077977_1").is_ok());
}

#[test]
fn test_zip_archive_reading_and_extraction() {
    let temp_dir = tempfile::tempdir().unwrap();
    let zip_path = temp_dir.path().join("test_photos.zip");

    // Create a zip with 1 valid photo and 1 non-photo text entry
    {
        let file = File::create(&zip_path).unwrap();
        let mut zip = ZipWriter::new(file);

        // Add valid photo container
        let mut jpeg = Vec::new();
        jpeg.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]);
        jpeg.extend_from_slice(b"JFIF\0\x01\x01\x01\0H\0H\0\0");
        jpeg.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x43, 0x00]);
        jpeg.extend_from_slice(&[16u8; 64]);
        jpeg.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x01, 0x00, 0x01, 0x00, 0x01, 0x01, 0x11, 0x00]);
        jpeg.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, 0x00, 0xFF, 0xD9]);

        let mut container = vec![0x12; 200];
        container.extend_from_slice(&jpeg);

        zip.start_file("PRDR001", SimpleFileOptions::default()).unwrap();
        zip.write_all(&container).unwrap();

        // Add text entry
        zip.start_file("notes.txt", SimpleFileOptions::default()).unwrap();
        zip.write_all(b"These are notes.").unwrap();

        zip.finish().unwrap();
    }

    let scanned = scan_zip_archive(&zip_path).unwrap();
    assert_eq!(scanned.len(), 2);

    let valid_photos: Vec<_> = scanned.iter().filter(|i| i.is_valid).collect();
    assert_eq!(valid_photos.len(), 1);
    assert_eq!(valid_photos[0].source.display_name(), "PRDR001");
}

#[test]
fn test_streaming_zip_export() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file1 = temp_dir.path().join("photo1.jpg");
    let file2 = temp_dir.path().join("photo2.jpg");

    std::fs::write(&file1, b"Test JPEG Bytes 1").unwrap();
    std::fs::write(&file2, b"Test JPEG Bytes 2").unwrap();

    let out_zip = temp_dir.path().join("output.zip");
    let mut exporter = StreamingZipExporter::new(&out_zip).unwrap();
    exporter.add_file_from_disk("photo1.jpg", &file1).unwrap();
    exporter.add_file_from_disk("photo2.jpg", &file2).unwrap();

    let written_bytes = exporter.finish().unwrap();
    assert!(written_bytes > 0);
    assert!(out_zip.exists());
}
