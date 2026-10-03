# Contributing to Blackwater

Thank you for your interest in contributing to **Blackwater**!

Blackwater is an open-source, offline-first native utility written in Rust, built with strict engineering principles:
1. **Extremely low memory footprint** ($O(1)$ relative to batch size).
2. **Zero network dependencies** (strictly local processing, no telemetry or cloud APIs).
3. **Byte-exact preservation** for JPEG extractions.

---

## Development Guidelines

### 1. Memory Budget Rules
- **Never** read entire batches of photos into memory simultaneously.
- **Never** keep decoded `DynamicImage` instances in memory beyond the scope of a single photo conversion.
- All file scanning and I/O pipelines must use buffered readers and writers with bounded chunk buffers (e.g. 64 KB).
- Bounded crossbeam channels (`capacity = 2`) must be used between scanner, processor, and UI to ensure backpressure.
- Thumbnail caches must stay strictly bounded with LRU eviction.

### 2. Preserving Offline Privacy
- Do **not** add any network crates, analytics, telemetry, crash reporting, or external API calls.
- Blackwater must compile and run flawlessly on machines without internet connectivity.

### 3. Running Tests & Benchmarks
Before submitting a pull request, ensure all tests and benchmarks pass:

```bash
# Run unit and integration tests
cargo test -- --nocapture

# Run benchmarks
cargo test --test memory_benchmark -- --nocapture
```

---

## Code Style
- Format your code with `cargo fmt`.
- Ensure zero compiler warnings with `cargo check` and `cargo clippy`.
