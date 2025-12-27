# rawthumb

Fast, extensible RAW thumbnail and preview extractor with optional auto-rotate and resize paths.

## Project Structure
- `src/` — core library code:
  - `exporter.rs` — high-level API (`get_thumbnail`, `export`, `export_to_file`) orchestrating decode, rotate, resize.
  - `core/` — shared types, errors, EXIF helpers, image helpers.
  - `makers/` — vendor-specific thumbnail extractors (Canon, Nikon, Sony, Olympus, Fuji, Panasonic, Adobe, etc.).
  - `formats/` — format preprocessors/fast-paths (e.g., CR3, Fuji fixes).
  - `image_resizer.rs` / `image_rotator.rs` — resizing and rotation utilities.
- `tests/` — integration tests and fixtures documentation (`tests/readme.md` for env setup).
- `Cargo.toml` — crate metadata and deps.

## Quick Start
```bash
cargo test
```
By default, fast unit/integration tests run and ignore long-running photo library scans.

### Running the ignored photo-library tests
These require a local RAW photo library and output directory. Set env vars (examples on Windows):
```powershell
setx RAWTHUMB_SCAN_ROOT "D:\Photos\Brands"
setx RAWTHUMB_OUTPUT_ROOT "D:\Photos\temp"
$env:RUST_LOG="rawthumb=debug"
```
Then run a specific ignored test, e.g.:
```bash
cargo test -- --nocapture --ignored --test export_thumbnails_tests export_thumbnails_from_photo_library
```
See `tests/readme.md` for more variants (DNG-specific, per-extension, etc.).

## Public API (library)
```rust
use rawthumb::ThumbnailExporter;
use rawthumb::export_config::ExportConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure auto-rotate and max size (pixels on longest edge)
    let config = ExportConfig::default()
        .with_auto_rotate(true)
        .with_max_border(Some(2048));

    let exporter = ThumbnailExporter::new_with_config(config);

    // Borrowed workflow: supply a buffer and get a thumbnail result.
    let raw_bytes = std::fs::read("photo.CR3")?;
    let thumb = exporter.get_thumbnail(&raw_bytes)?;
    println!("jpeg bytes: {}", thumb.jpeg.len());

    // File-to-file convenience:
    exporter.export_to_file("photo.CR3", "thumb.jpg")?;

    // File-to-owned-thumbnail convenience (detaches from mmap):
    let owned = exporter.export("photo.CR3")?;
    println!("rotated: {}, resized: {}", owned.is_rotated, owned.is_resized);

    Ok(())
}
```
Key return type: `ThumbnailResult` (JPEG bytes as `Cow<[u8]>`, orientation, and flags `is_rotated` / `is_resized`).

## Notes
- Input reading uses mmap for speed; resizing leverages `fast_image_resize` and TurboJPEG scaling; rotation uses TurboJPEG when possible.
- SIMD is enabled when available (SSE4.1 / AVX2) via `fast_image_resize`.
- Optional Rayon is used in dev-only parallel smoke tests. No runtime dependency in the core library.૮

## Performance (sample)
- Parallel photo library run (Rayon, 8 threads): 394 thumbnails, 0 failures; total 3.897s, avg ~9.9ms/thumb. Results will vary with hardware and IO bandwidth.
