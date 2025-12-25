use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::time::Instant;

use rawthumb::ThumbnailExporter;
use rawthumb::core::image_format::ImageFormt;

fn init_logger() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // Default to chatty logs for this test suite but still respect RUST_LOG when set.
        let env = env_logger::Env::default()
            .default_filter_or("rawthumb=debug,export_thumbnails_from_raw=debug");
        env_logger::Builder::from_env(env)
            .target(env_logger::Target::Stdout)
            .format_timestamp_secs()
            .format_target(true)
            .is_test(true)
            .filter_level(log::LevelFilter::Trace)
            .try_init()
            .ok();
    });
}

#[test]
#[ignore]
fn export_thumbnails_from_photo_library() -> Result<(), Box<dyn std::error::Error>> {
    init_logger();
    let start = Instant::now();

    let scan_root = env::var("RAWTHUMB_SCAN_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"D:\Photos\Brands"));

    if !scan_root.exists() {
        eprintln!(
            "scan root {:?} not found; set RAWTHUMB_SCAN_ROOT to your library and rerun",
            scan_root
        );
        return Ok(());
    }

    let files = scan_supported_files(&scan_root)?;
    log::debug!(
        " 🟢 Found {} supported RAW files under path {:?}",
        files.len(),
        scan_root
    );
    if files.is_empty() {
        return Ok(());
    }

    // Determine output root from env or default to the requested path.
    let output_root = env::var("RAWTHUMB_OUTPUT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"D:\Photos\temp"));
    fs::create_dir_all(&output_root)?;
    log::debug!(
        " 🟢 Exporting thumbnails into {}",
        output_root.to_string_lossy().to_string()
    );

    let mut successes = 0usize;
    let mut failures: Vec<(PathBuf, String)> = Vec::new();

    let exporter = ThumbnailExporter::new();
    for path in files {
        match process_file(&path, &scan_root, &output_root, &exporter) {
            Ok(()) => {
                successes += 1;
            }
            Err(e) => {
                log::error!("Failed to export thumbnail for {}: {:?}", path.display(), e);
                failures.push((path.clone(), e.to_string()));
            }
        }
    }

    log::debug!(
        " 🟢 Summary: {} succeeded, {} failed; outputs at {:?}, total time: {:?}, average: {:?}",
        successes,
        failures.len(),
        output_root,
        start.elapsed(),
        start.elapsed() / (successes as u32 + failures.len() as u32)
    );
    log::debug!("=========================");

    if !failures.is_empty() {
        let messages = failures
            .iter()
            .map(|(p, e)| format!("{}: {}", p.to_string_lossy().to_string(), e))
            .collect::<Vec<_>>()
            .join("\n");
        // return Err(format!(" 💥 Some files failed:\n{}", messages).into());
        for message in messages.lines() {
            eprintln!(" 💥 {}", message);
        }
    }

    Ok(())
}

#[test]
#[ignore]
fn export_thumbnails_test() -> Result<(), Box<dyn std::error::Error>> {
    init_logger();
    let start = Instant::now();
    let root = PathBuf::from(r"D:\Photos\Brands");
    let test_files = vec![
        root.join("Canon").join("EOS 90D").join("IMG_4011.cr3"),
        root.join("Canon")
            .join("EOS 7D")
            .join("Canon - EOS 7D - RAW (3_2).CR2"),
        root.join("Nikon").join("D500").join("DSC_1284.NEF"),
    ];

    let output_root = PathBuf::from(r"D:\Photos\temp\thumbnails");
    fs::create_dir_all(&output_root)?;

    let mut successes = 0usize;
    let mut failures: Vec<(PathBuf, String)> = Vec::new();

    let exporter = ThumbnailExporter::new();
    for path in test_files {
        match process_file(&path, &root, &output_root, &exporter) {
            Ok(()) => {
                successes += 1;
            }
            Err(e) => {
                log::error!("Failed to export thumbnail for {}: {:?}", path.display(), e);
                failures.push((path.clone(), e.to_string()));
            }
        }
    }

    log::debug!(
        " 🟢 Summary: {} succeeded, {} failed; outputs at {:?}, total time: {:?}, average: {:?}",
        successes,
        failures.len(),
        output_root,
        start.elapsed(),
        start.elapsed() / (successes as u32 + failures.len() as u32)
    );
    log::debug!("=========================");

    Ok(())
}

#[test]
#[ignore]
fn export_thumbnails_from_dng_test() -> Result<(), Box<dyn std::error::Error>> {
    init_logger();

    log::info!("Starting export_thumbnails_from_dng_test");
    let start = Instant::now();
    let root = PathBuf::from(r"D:\Photos\Brands");
    let test_files = vec![
        root.join("Canon")
            .join("EOS R")
            .join("Canon-eos-r-raw-00004.cr3"),
        root.join("Sony")
            .join("A1")
            .join("tag @ryanbreitkreutz - free raws from @signatureeditsco - DSC06683.dng"),
        root.join("Sony")
            .join("A1")
            .join("tag @ryanbreitkreutz - free raws from @signatureeditsco - DSC06780.dng"),
        root.join("Sony")
            .join("A1")
            .join("tag @ryanbreitkreutz - free raws from @signatureeditsco - DSC07073.dng"),
        root.join("DJ").join("DJI-mavic-2-pro-raw-00005.dng"),
        root.join("DJ").join("DJI-mavic-2-pro-raw-00008.dng"),
        root.join("DJ").join("DJI-mavic-2-pro-raw-00007.dng"),
        root.join("OM System").join("P8206009.ORF"),
        root.join("OM System").join("PA086285.ORF"),
        root.join("OM System").join("PA016098.ORF"),
    ];

    let output_root = PathBuf::from(r"D:\Photos\temp\thumbnails\dng");
    fs::create_dir_all(&output_root)?;

    let mut successes = 0usize;
    let mut failures: Vec<(PathBuf, String)> = Vec::new();

    let exporter = ThumbnailExporter::new();
    for path in test_files {
        match process_file(&path, &root, &output_root, &exporter) {
            Ok(()) => {
                successes += 1;
            }
            Err(e) => {
                log::error!("Failed to export thumbnail for {}: {:?}", path.display(), e);
                failures.push((path.clone(), e.to_string()));
            }
        }
    }

    log::debug!(
        " 🟢 Summary: {} succeeded, {} failed; outputs at {:?}, total time: {:?}, average: {:?}",
        successes,
        failures.len(),
        output_root,
        start.elapsed(),
        start.elapsed() / (successes as u32 + failures.len() as u32)
    );
    log::debug!("=========================");

    Ok(())
}

fn scan_supported_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut stack = vec![root.to_path_buf()];
    let mut results = Vec::new();

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() && ImageFormt::is_supported_path(&path) {
                results.push(path);
            }
        }
    }

    Ok(results)
}

fn process_file(
    path: &Path,
    root: &Path,
    output_root: &Path,
    exporter: &ThumbnailExporter,
) -> Result<(), Box<dyn std::error::Error>> {
    // log::debug!("➡️  Processing file {}", path.to_string_lossy().to_string());
    log::debug!("➡️  Processing file {}", path.to_string_lossy().to_string());
    let raw_bytes = fs::read(path)?;

    // Build an output path that mirrors the input structure and uses .jpg extension.
    let relative = path.strip_prefix(root).unwrap_or(path);
    let output_path = output_root.join(relative).with_extension("jpg");
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let output_path_str = output_path.to_string_lossy().to_string();
    exporter.export_thumbnail_to_file(&raw_bytes, &output_path_str)?;

    // Basic validation of the output JPEG.
    let output_bytes = fs::read(&output_path)?;
    if output_bytes.len() < 2 {
        return Err(" 💥 Exported JPEG is too small".into());
    }
    if output_bytes[0] != 0xFF || output_bytes[1] != 0xD8 {
        return Err(" 💥 Missing JPEG SOI marker".into());
    }
    if output_bytes[output_bytes.len() - 2] != 0xFF || output_bytes[output_bytes.len() - 1] != 0xD9
    {
        return Err(" 💥 Missing JPEG EOI marker".into());
    }

    Ok(())
}
