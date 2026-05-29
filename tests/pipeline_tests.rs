use sonic_bridge::config::SonicConfig;
use sonic_bridge::pipeline::SonicPipeline;
use std::fs;
use std::path::Path;

fn ensure_mock_wav(path: &Path) {
    if path.exists() {
        return;
    }
    let fixtures_dir = path.parent().unwrap();
    if !fixtures_dir.exists() {
        fs::create_dir_all(fixtures_dir).unwrap();
    }
    // Generate a 1.5 seconds sine wave to have enough frames for onset mode and multiple chunks
    let sample_rate: u32 = 44100;
    let duration = 1.5f32;
    let freq = 440.0f32;
    let num_samples = (duration * sample_rate as f32) as usize;
    let mut data = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        // Inject a few sudden onset peaks at 0.5s and 1.0s to trigger onset detector
        let mut envelope = 1.0f32;
        if (t - 0.5).abs() < 0.05 || (t - 1.0).abs() < 0.05 {
            envelope = 10.0f32;
        }
        let val = (2.0 * std::f32::consts::PI * freq * t).sin() * envelope;
        let sample = (val.clamp(-1.0, 1.0) * 32767.0) as i16;
        data.extend_from_slice(&sample.to_le_bytes());
    }

    let mut file = fs::File::create(path).unwrap();
    let subchunk2_size = data.len() as u32;
    let chunk_size = 36 + subchunk2_size;

    use std::io::Write;
    file.write_all(b"RIFF").unwrap();
    file.write_all(&chunk_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&sample_rate.to_le_bytes()).unwrap();
    let byte_rate = sample_rate * 2;
    file.write_all(&byte_rate.to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&subchunk2_size.to_le_bytes()).unwrap();
    file.write_all(&data).unwrap();
}

#[test]
fn test_pipeline_with_custom_config() {
    let wav_path = Path::new("tests/fixtures/mock.wav");
    ensure_mock_wav(wav_path);

    let custom_config = SonicConfig {
        step_size: 1.0,
        onset_mode: true,
        onset_threshold: 0.1,
        ..Default::default()
    };

    let (meta, segs) = SonicPipeline::process_single(wav_path, &custom_config).unwrap();

    assert!(meta.duration_seconds > 0.0);
    assert!(!segs.is_empty());

    // Check that segments are created and have valid description values
    for seg in &segs {
        assert!(!seg.time_range.is_empty());
        assert!(!seg.dynamic_level.is_empty());
        assert!(!seg.timbre_brightness.is_empty());
        assert!(!seg.rhythm_activity.is_empty());
    }
}
