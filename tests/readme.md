setx RAWTHUMB_SCAN_ROOT "D:\Photos\Brands"
setx RAWTHUMB_OUTPUT_ROOT "D:\Photos\temp"

cargo test -- --nocapture --ignored --test export_thumbnails_from_raw export_thumbnails_from_photo_library

cargo test -- --nocapture --ignored --test export_thumbnails_from_raw export_thumbnails_test

cargo test -- --nocapture --test export_api_characterization export_thumbnail_public_api_smoke