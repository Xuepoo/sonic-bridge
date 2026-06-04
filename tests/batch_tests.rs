use sonic_bridge::batch::{get_target_report_path, run_batch, scan_directory};
use sonic_bridge::config::SonicConfig;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn generate_sine_wav(
    path: &Path,
    duration: f32,
    sample_rate: u32,
    freq: f32,
) -> std::io::Result<()> {
    let num_samples = (duration * sample_rate as f32) as usize;
    let mut data = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let val = (2.0 * std::f32::consts::PI * freq * t).sin();
        let sample = (val * 32767.0) as i16;
        data.extend_from_slice(&sample.to_le_bytes());
    }

    let mut file = File::create(path)?;
    let subchunk2_size = data.len() as u32;
    let chunk_size = 36 + subchunk2_size;

    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&chunk_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt subchunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // subchunk1_size
    file.write_all(&1u16.to_le_bytes())?; // audio_format = 1 (PCM)
    file.write_all(&1u16.to_le_bytes())?; // num_channels = 1
    file.write_all(&sample_rate.to_le_bytes())?; // sample_rate
    let byte_rate = sample_rate * 2;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?; // block_align = num_channels * bits_per_sample/8
    file.write_all(&16u16.to_le_bytes())?; // bits_per_sample = 16

    // data subchunk
    file.write_all(b"data")?;
    file.write_all(&subchunk2_size.to_le_bytes())?;
    file.write_all(&data)?;

    Ok(())
}

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

#[test]
fn test_run_batch_execution() {
    let temp_dir = Path::new("tests/fixtures/temp_batch_run");
    fs::create_dir_all(temp_dir).unwrap();

    // Create 2 real temporary WAV files (short, 0.1s)
    let song1 = temp_dir.join("song1.wav");
    let song2 = temp_dir.join("song2.wav");
    generate_sine_wav(&song1, 0.1, 22050, 440.0).unwrap();
    generate_sine_wav(&song2, 0.1, 22050, 440.0).unwrap();

    let config = SonicConfig {
        jobs: Some(2),
        extensions: vec!["wav".to_string()],
        skip_existing: false,
        ..Default::default()
    };

    // Execute run_batch with dry_run = false, no_progress = true
    let summary = run_batch(&config, temp_dir, false, true).unwrap();
    assert_eq!(summary.total, 2);
    assert_eq!(summary.processed, 2);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.errors.len(), 0);

    // Verify report output files exist
    let report1 = temp_dir.join("song1.wav.lrmd.md");
    let report2 = temp_dir.join("song2.wav.lrmd.md");
    assert!(report1.exists());
    assert!(report2.exists());

    // Verify content of one report file
    let content = fs::read_to_string(report1).unwrap();
    assert!(content.contains("LLM-Readable Music Descriptor"));
    assert!(content.contains("song1.wav"));

    fs::remove_dir_all(temp_dir).unwrap();
}
