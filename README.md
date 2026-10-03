# Blackwater

> **Private, offline-first RDR2 photo extraction and conversion utility.**

Blackwater is a native desktop application written in Rust designed to extract and convert photographs from Red Dead Redemption 2 (PC) camera and photomode files with minimal RAM consumption, zero telemetry, and maximum preservation of original image data.

---

## Key Features

- 🔒 **100% Offline & Private ("LOCAL ONLY")**: Your photos never leave your machine. No accounts, no telemetry, no analytics, no cloud APIs, and no network requests.
- ⚡ **Extremely Low Memory Footprint**: Engineered specifically for systems with 4 GB to 8 GB RAM. Memory usage remains bounded in $O(1)$ regardless of whether you process 10 photos or 10,000 photos.
- 🎯 **Byte-Exact JPEG Extraction**: When extracting to JPEG, Blackwater directly streams the original embedded JPEG byte stream from the game container to disk. **Zero decoding or re-compression** is performed, preserving 100% original quality and metadata.
- 🖼️ **Lossless PNG Conversion**: Converts extracted JPEGs to PNG sequentially (one image in RAM at a time) with strict memory thresholds.
- 📦 **Streaming Archive Support**: Direct inspection and extraction from `.zip` archives with path traversal protection, plus incremental single-pass ZIP export.
- 🖥️ **Native Windows GUI**: Built with pure Rust native GUI frameworks (no Electron, no WebView, no Chromium, no Node.js).
- 📊 **Real Memory & Hardware Monitoring**: Real-time process working set monitoring using native Windows kernel APIs and automatic hardware RAM tier detection.

---

## How RDR2 Photo Extraction Works

Red Dead Redemption 2 stores in-game camera snapshots and Photo Mode images in proprietary Rockstar container files (usually named `PRDR<number>` without an extension) located in:

```text
%USERPROFILE%\Documents\Rockstar Games\Red Dead Redemption 2\Profiles\<ProfileID>\
```

These files start with a proprietary header containing in-game camera parameters and metadata. Embedded within each container is a standard JFIF/Exif JPEG stream:
1. **SOI (Start Of Image)** marker: `0xFF, 0xD8, 0xFF`
2. **SOF (Start Of Frame)** headers: Containing image resolution and components.
3. **EOI (End Of Image)** marker: `0xFF, 0xD9`

Blackwater scans the binary contents of input files using buffered chunking, locates the embedded JPEG stream, and extracts it with byte-exact fidelity.

---

## Important Quality Rule: JPEG vs PNG

### JPEG Output (Recommended)
When **JPEG** is chosen:
```text
RDR2 Container ──> [ Direct Byte Stream Copy ] ──> Output JPEG (.jpg)
```
The application **does not** decode or re-encode the image. The original JPEG compressed by the game engine is saved directly to disk without any generation loss.

### PNG Output
When **PNG** is chosen:
```text
RDR2 Container ──> Embedded JPEG ──> JPEG Decoder ──> Pixel Buffer ──> PNG Encoder ──> Output PNG (.png)
```

> [!NOTE]
> **PNG is lossless, but it cannot restore information already discarded by the original JPEG compression.**
> Converting an embedded JPEG to PNG produces a lossless file representation of the decoded pixels, but it cannot improve or sharpen the original source quality.

---

## Low-Memory Architecture & Pipeline

Unlike traditional image converters that load thousands of images into RAM simultaneously, Blackwater enforces strict bounded pipeline stages:

```text
Disk / ZIP
    │
    ▼  (Bounded Channel, cap = 2)
[ Streaming Scanner ]
    │
    ▼  (Bounded Channel, cap = 2)
[ Single Worker Pool ] (1 image decoded at a time)
    │
    ▼  (Buffered Write / Direct Stream)
Output File / Streaming ZIP Exporter
    │
    ▼
[ Free Buffers & Drop Pixel Memory ]
```

### Memory Modes

| Mode | Workers | Thumbnail Cache | RAM Working Set (Target) |
| :--- | :---: | :---: | :---: |
| **LOW MEMORY** (Default) | 1 | 20 max (320px) | **< 100 MB** (JPEG) / **< 300 MB** (PNG) |
| **BALANCED** | 2 | 40 max (320px) | **< 450 MB** |

*On systems with 8 GB RAM or less, Blackwater automatically detects hardware physical memory and enables LOW MEMORY mode.*

---

## Building from Source

### Prerequisites
- [Rust toolchain](https://rustup.rs/) (Rust 1.75+ or newer recommended)
- Windows 10/11 (or Linux with X11/Wayland dependencies)

### Compile and Run

```bash
# Clone repository
git clone https://github.com/your-username/blackwater.git
cd blackwater

# Check and build dev executable
cargo build

# Run native application
cargo run --release
```

---

## Running Tests & Benchmarks

Blackwater includes a comprehensive suite of automated tests for marker detection, byte-exact extraction, security traversal prevention, directory recursion, and memory benchmarks.

```bash
# Run all unit and integration tests
cargo test -- --nocapture

# Run dedicated memory scaling benchmarks (100, 200, 1000 photos)
cargo test --test memory_benchmark -- --nocapture
```

---

## Disclaimer

**Blackwater** is an independent, open-source utility and is **not** affiliated with, endorsed by, or sponsored by Rockstar Games, Take-Two Interactive, or any of their subsidiaries. All trademarks and game titles belong to their respective owners.

---

## License

This project is licensed under the [MIT License](LICENSE).
