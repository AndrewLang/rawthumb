use std::fs;
use std::path::Path;

use tempfile::TempDir;

/// Characterization test for `export_thumbnail_data` and `export_thumbnail_to_file` using RAW fixtures.
#[test]
fn export_thumbnail_public_api_smoke() -> Result<(), Box<dyn std::error::Error>> {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/raw");
    if !fixtures_dir.exists() {
        eprintln!("fixtures directory missing; skipping");
        return Ok(());
    }

    let entries = fs::read_dir(&fixtures_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|ft| ft.is_file()).unwrap_or(false))
        .collect::<Vec<_>>();
    if entries.is_empty() {
        eprintln!("no RAW fixtures found; skipping");
        return Ok(());
    }

    let output_root = TempDir::new()?;

    for entry in entries {
        let raw_path = entry.path();
        let raw_bytes = fs::read(&raw_path)?;

        // export_thumbnail_data should return JPEG bytes with SOI/EOI markers.
        let exporter = rawthumb::ThumbnailExporter::new();
        let data = exporter.export_thumbnail_data(&raw_bytes)?;
        assert!(
            data.len() >= 2,
            "export_thumbnail_data returned too few bytes for {:?}",
            raw_path
        );
        assert_eq!(
            data[0], 0xFF,
            "missing JPEG SOI marker 0xFF for {:?}",
            raw_path
        );
        assert_eq!(
            data[1], 0xD8,
            "missing JPEG SOI marker 0xD8 for {:?}",
            raw_path
        );
        assert_eq!(
            data[data.len() - 2],
            0xFF,
            "missing JPEG EOI marker 0xFF for {:?}",
            raw_path
        );
        assert_eq!(
            data[data.len() - 1],
            0xD9,
            "missing JPEG EOI marker 0xD9 for {:?}",
            raw_path
        );

        // export_thumbnail_to_file should write a JPEG file we can read back and validate.
        let output_path = output_root
            .path()
            .join(entry.file_name())
            .with_extension("jpg");
        let output_path_str = output_path.to_string_lossy().to_string();
        let exporter = rawthumb::ThumbnailExporter::new();
        exporter.export_thumbnail_to_file(&raw_bytes, &output_path_str)?;

        let written = fs::read(&output_path)?;
        assert!(
            written.len() >= 2,
            "exported file too small for {:?}",
            raw_path
        );
        assert_eq!(
            written[0], 0xFF,
            "missing JPEG SOI marker 0xFF for {:?}",
            raw_path
        );
        assert_eq!(
            written[1], 0xD8,
            "missing JPEG SOI marker 0xD8 for {:?}",
            raw_path
        );
        assert_eq!(
            written[written.len() - 2],
            0xFF,
            "missing JPEG EOI marker 0xFF for {:?}",
            raw_path
        );
        assert_eq!(
            written[written.len() - 1],
            0xD9,
            "missing JPEG EOI marker 0xD9 for {:?}",
            raw_path
        );
    }

    Ok(())
}
