use sonic_bridge::batch::{get_target_report_path, scan_directory};
use std::fs;
use std::path::Path;

#[test]
fn test_directory_scanning_and_path_mapping() {
    let temp_dir = Path::new("tests/fixtures/temp_batch_test");
    fs::create_dir_all(temp_dir.join("subdir")).unwrap();

    fs::write(temp_dir.join("track1.mp3"), b"").unwrap();
    fs::write(temp_dir.join("track2.wav"), b"").unwrap();
    fs::write(temp_dir.join("subdir/track3.flac"), b"").unwrap();
    fs::write(temp_dir.join("readme.txt"), b"").unwrap();

    let extensions = vec!["mp3".to_string(), "flac".to_string(), "wav".to_string()];
    let mut files = Vec::new();
    scan_directory(temp_dir, &extensions, &mut files).unwrap();

    // Sort files to ensure deterministic index access in assertion
    files.sort();

    assert_eq!(files.len(), 3);

    let mapped = get_target_report_path(&files[0], temp_dir, Some("tests/fixtures/out"));
    assert!(mapped.to_string_lossy().contains("tests/fixtures/out"));
    assert!(mapped.to_string_lossy().ends_with(".lrmd.md"));

    fs::remove_dir_all(temp_dir).unwrap();
}
