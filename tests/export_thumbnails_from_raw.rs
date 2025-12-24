use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

const SUPPORTED_EXTS: &[&str] = &[
    "cr2", "cr3", "nef", "raf", "arw", "orf", "rw2", "dng", "raw",
];

#[test]
#[ignore]
fn export_thumbnails_from_photo_library() -> Result<(), Box<dyn std::error::Error>> {
    // Allow overriding the scan root via env var for portability; default to the requested path.
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
    println!(
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
    println!(" 🟢 Exporting thumbnails into {:?}", output_root);

    let mut successes = 0usize;
    let mut failures: Vec<(PathBuf, String)> = Vec::new();

    for path in files {
        match process_file(&path, &scan_root, &output_root) {
            Ok(()) => {
                successes += 1;
            }
            Err(e) => {
                failures.push((path.clone(), e.to_string()));
            }
        }
    }

    println!(
        " 🟢 Summary: {} succeeded, {} failed; outputs at {:?}, total time: {:?}, average: {:?}",
        successes,
        failures.len(),
        output_root,
        start.elapsed(),
        start.elapsed() / (successes as u32 + failures.len() as u32)
    );
    println!("=========================");

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
    let start = Instant::now();
    let root = PathBuf::from(r"D:\Photos\Brands");
    let test_files = vec![
        root.join("Cannon").join("EOS 90D").join("IMG_4011.cr3"),
        root.join("Cannon")
            .join("EOS 7D")
            .join("Canon - EOS 7D - RAW (3_2).CR2"),
        root.join("Nikon").join("D500").join("DSC_1284.NEF"),
    ];

    let output_root = PathBuf::from(r"D:\Photos\temp\thumbnails");
    fs::create_dir_all(&output_root)?;

    let mut successes = 0usize;
    let mut failures: Vec<(PathBuf, String)> = Vec::new();

    for path in test_files {
        match process_file(&path, &root, &output_root) {
            Ok(()) => {
                successes += 1;
            }
            Err(e) => {
                failures.push((path.clone(), e.to_string()));
            }
        }
    }

    println!(
        " 🟢 Summary: {} succeeded, {} failed; outputs at {:?}, total time: {:?}, average: {:?}",
        successes,
        failures.len(),
        output_root,
        start.elapsed(),
        start.elapsed() / (successes as u32 + failures.len() as u32)
    );
    println!("=========================");

    Ok(())
}

/// Recursively scan the root directory for RAW files with supported extensions.
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
            } else if file_type.is_file() && is_supported(&path) {
                results.push(path);
            }
        }
    }

    Ok(results)
}

fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_ascii_lowercase();
            SUPPORTED_EXTS.iter().any(|e| *e == lower)
        })
        .unwrap_or(false)
}

/// Process a single RAW file: export its thumbnail into the output root, mirroring the relative path.
fn process_file(
    path: &Path,
    root: &Path,
    output_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let raw_bytes = fs::read(path)?;

    // Build an output path that mirrors the input structure and uses .jpg extension.
    let relative = path.strip_prefix(root).unwrap_or(path);
    let output_path = output_root.join(relative).with_extension("jpg");
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let output_path_str = output_path.to_string_lossy().to_string();
    rawthumb::export_thumbnail_to_file(&raw_bytes, &output_path_str)?;

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
