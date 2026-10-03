use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;
use blackwater::models::{OutputFormat, PhotoMetadata, PhotoSource};
use blackwater::processor::queue::{ProcessJob, WorkerEvent};
use blackwater::processor::run_worker_pool;
use blackwater::settings::get_process_memory_usage;

/// Creates a synthetic minimal JPEG for testing.
fn create_test_container() -> Vec<u8> {
    let mut container = vec![0xEE; 300]; // 300 bytes header
    // Minimal JPEG
    container.extend_from_slice(&[
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10,
        0x4A, 0x46, 0x49, 0x46, 0x00, 0x01, 0x01, 0x01, 0x00, 0x48, 0x00, 0x48, 0x00, 0x00,
        0xFF, 0xDB, 0x00, 0x43, 0x00,
    ]);
    container.extend_from_slice(&[16u8; 64]);
    container.extend_from_slice(&[
        0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x02, 0x00, 0x02, 0x00, 0x01, 0x01, 0x11, 0x00,
        0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00,
        0x00, 0xFF, 0xD9,
    ]);
    container.extend_from_slice(&[0xFF; 50]); // 50 bytes trailing
    container
}

fn run_batch_benchmark(photo_count: usize) -> (u128, u64, u64) {
    let temp_dir = tempfile::tempdir().unwrap();
    let sample_container = create_test_container();
    let container_path = temp_dir.path().join("sample_rdr2_photo");
    std::fs::write(&container_path, &sample_container).unwrap();

    let meta = PhotoMetadata {
        width: 512,
        height: 512,
        jpeg_offset: 300,
        jpeg_len: 99,
        source_size_bytes: sample_container.len() as u64,
        output_size_bytes: 99,
    };

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let pause_flag = Arc::new(AtomicBool::new(false));

    let (jobs_tx, jobs_rx) = crossbeam_channel::bounded::<ProcessJob>(2);
    let (events_tx, events_rx) = crossbeam_channel::bounded::<WorkerEvent>(2);

    let start_time = Instant::now();
    let cancel_worker = Arc::clone(&cancel_flag);
    let pause_worker = Arc::clone(&pause_flag);

    let worker_thread = std::thread::spawn(move || {
        run_worker_pool(jobs_rx, events_tx, cancel_worker, pause_worker, photo_count);
    });

    let feeder_path = container_path.clone();
    let feeder_dir = temp_dir.path().to_path_buf();
    let feeder_meta = meta.clone();

    let feeder_thread = std::thread::spawn(move || {
        for i in 0..photo_count {
            let out_file = feeder_dir.join(format!("photo_{:04}.jpg", i + 1));
            let job = ProcessJob {
                index: i,
                source: PhotoSource::LocalFile(feeder_path.clone()),
                metadata: feeder_meta.clone(),
                output_path: out_file,
                format: OutputFormat::Jpeg,
                max_memory_mb: 256,
            };
            if jobs_tx.send(job).is_err() {
                break;
            }
        }
    });

    let mut success_count = 0;
    let mut total_output_bytes = 0u64;

    while let Ok(event) = events_rx.recv() {
        match event {
            WorkerEvent::JobFinished(photo) => {
                if photo.status == blackwater::models::ProcessStatus::Success {
                    success_count += 1;
                    total_output_bytes += photo.metadata.map(|m| m.output_size_bytes).unwrap_or(0);
                }
            }
            WorkerEvent::BatchFinished { .. } => break,
            _ => {}
        }
    }

    feeder_thread.join().unwrap();
    worker_thread.join().unwrap();

    let duration_ms = start_time.elapsed().as_millis();
    let peak_ram_bytes = get_process_memory_usage().map(|(_, peak)| peak).unwrap_or(0);

    assert_eq!(success_count, photo_count);
    (duration_ms, peak_ram_bytes, total_output_bytes)
}

#[test]
fn benchmark_100_photos() {
    let (time_ms, peak_ram, out_bytes) = run_batch_benchmark(100);
    println!("\n[BENCHMARK 100 PHOTOS]");
    println!("Time: {} ms", time_ms);
    println!("Peak RAM: {:.2} MB", peak_ram as f64 / (1024.0 * 1024.0));
    println!("Output: {} bytes\n", out_bytes);
}

#[test]
fn benchmark_200_photos() {
    let (time_ms, peak_ram, out_bytes) = run_batch_benchmark(200);
    println!("\n[BENCHMARK 200 PHOTOS]");
    println!("Time: {} ms", time_ms);
    println!("Peak RAM: {:.2} MB", peak_ram as f64 / (1024.0 * 1024.0));
    println!("Output: {} bytes\n", out_bytes);
}

#[test]
fn benchmark_1000_photos_o1_memory_verification() {
    let (time_ms_100, peak_ram_100, _) = run_batch_benchmark(100);
    let (time_ms_1000, peak_ram_1000, out_bytes_1000) = run_batch_benchmark(1000);

    println!("\n=======================================================");
    println!("  BLACKWATER LOW-MEMORY O(1) BENCHMARK VERIFICATION");
    println!("=======================================================");
    println!("Batch 100  photos: {:>5} ms | Peak RAM: {:.2} MB", time_ms_100, peak_ram_100 as f64 / (1024.0 * 1024.0));
    println!("Batch 1000 photos: {:>5} ms | Peak RAM: {:.2} MB", time_ms_1000, peak_ram_1000 as f64 / (1024.0 * 1024.0));
    println!("Total 1000 Output: {} bytes", out_bytes_1000);
    println!("=======================================================\n");

    // Verification: Peak RAM between 100 and 1,000 photos should be virtually identical
    // (O(1) memory complexity with respect to batch size, well within margin of OS allocator)
    if peak_ram_100 > 0 && peak_ram_1000 > 0 {
        let diff_mb = ((peak_ram_1000 as i64 - peak_ram_100 as i64).abs() as f64) / (1024.0 * 1024.0);
        println!("RAM Difference between 100 and 1,000 photos: {:.2} MB", diff_mb);
        assert!(diff_mb < 50.0, "Memory must remain bounded in O(1) and not scale with batch size!");
    }
}
