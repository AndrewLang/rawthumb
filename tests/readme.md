setx RAWTHUMB_SCAN_ROOT "D:\Photos\Brands"
setx RAWTHUMB_OUTPUT_ROOT "D:\Photos\temp"

$env:RUST_LOG="rawthumb=debug"

cargo test -- --nocapture --ignored --test export_thumbnails_tests export_thumbnails_from_photo_library

cargo test -- --nocapture --ignored --test export_thumbnails_tests export_thumbnails_test

cargo test -- --nocapture --ignored --test export_thumbnails_tests export_thumbnails_from_dng_test

cargo test -- --nocapture --test export_api_characterization export_thumbnail_public_api_smoke


TODO:
1. Provide supported image formats
2. Config for auto rotate
3. Improve performance for cr3 format (done)