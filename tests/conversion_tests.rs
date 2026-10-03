use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use blackwater::filesystem::resolve_output_path;
use blackwater::models::{OutputFormat, PhotoMetadata, PhotoSource};
use blackwater::processor::queue::{ProcessJob, WorkerEvent};
use blackwater::processor::run_worker_pool;

#[test]
fn test_output_naming_and_collision_resolution() {
    let temp_dir = tempfile::tempdir().unwrap();
    let base_out = temp_dir.path();

    let source = PhotoSource::LocalFile(PathBuf::from("C:/Games/PRDR3852077977_1"));

    // 1. First resolution with preservation
    let path1 = resolve_output_path(
        base_out,
        &source,
        OutputFormat::Jpeg,
        true,  // preserve filenames
        false, // preserve folder
        false, // overwrite
        0,
        None,
    );
    assert_eq!(path1.file_name().unwrap(), "PRDR3852077977_1.jpg");

    // Create file at path1 to trigger collision
    std::fs::write(&path1, b"existing").unwrap();

    // 2. Second resolution should resolve to (1).jpg
    let path2 = resolve_output_path(
        base_out,
        &source,
        OutputFormat::Jpeg,
        true,
        false,
        false,
        0,
        None,
    );
    assert_eq!(path2.file_name().unwrap(), "PRDR3852077977_1 (1).jpg");

    // 3. Format override without preservation -> PHOTO_0001.png
    let path3 = resolve_output_path(
        base_out,
        &source,
        OutputFormat::Png,
        false, // do not preserve
        false,
        false,
        0,
        None,
    );
    assert_eq!(path3.file_name().unwrap(), "PHOTO_0001.png");
}

#[test]
fn test_cancellation_and_partial_cleanup() {
    let temp_dir = tempfile::tempdir().unwrap();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let pause_flag = Arc::new(AtomicBool::new(false));

    let (jobs_tx, jobs_rx) = crossbeam_channel::bounded::<ProcessJob>(2);
    let (events_tx, events_rx) = crossbeam_channel::bounded::<WorkerEvent>(2);

    let cancel_clone = Arc::clone(&cancel_flag);
    let pause_clone = Arc::clone(&pause_flag);

    // Spawn worker
    let handle = std::thread::spawn(move || {
        run_worker_pool(jobs_rx, events_tx, cancel_clone, pause_clone, 5);
    });

    // Send 1 job, then cancel
    let out_file = temp_dir.path().join("photo_cancelled.jpg");
    let job = ProcessJob {
        index: 0,
        source: PhotoSource::LocalFile(PathBuf::from("non_existent_source")),
        metadata: PhotoMetadata {
            width: 1920,
            height: 1080,
            jpeg_offset: 0,
            jpeg_len: 100,
            source_size_bytes: 100,
            output_size_bytes: 100,
        },
        output_path: out_file.clone(),
        format: OutputFormat::Jpeg,
        max_memory_mb: 256,
    };

    jobs_tx.send(job).unwrap();
    cancel_flag.store(true, Ordering::Relaxed);
    drop(jobs_tx); // Close queue

    handle.join().unwrap();

    // Consume finish event
    let mut finished = false;
    while let Ok(evt) = events_rx.recv() {
        if let WorkerEvent::BatchFinished { cancelled, .. } = evt {
            assert!(cancelled);
            finished = true;
        }
    }
    assert!(finished);
}
