setx RAWTHUMB_SCAN_ROOT "D:\Photos\Brands"
setx RAWTHUMB_OUTPUT_ROOT "D:\Photos\temp"

$env:RUST_LOG="rawthumb=debug"

cargo test -- --nocapture --ignored --test export_thumbnails_tests export_thumbnails_from_photo_library

cargo test -- --nocapture --ignored --test export_thumbnails_tests export_thumbnails_test

cargo test -- --nocapture --ignored --test export_thumbnails_tests export_thumbnails_from_dng_test

cargo test -- --nocapture --ignored --test export_thumbnails_tests export_thumbnails_for_specific_ext

cargo test -- --nocapture --test export_thumbnails_tests read_orientation_from_raw_fixture

cargo test -- --nocapture --test export_api_characterization export_thumbnail_public_api_smoke


TODO:
1. Provide supported image formats (done)
2. Config for auto rotate (done)
3. Config for auto resize (done)
4. Improve performance for cr3 format (done)
5. Update API with more friendly name (done)