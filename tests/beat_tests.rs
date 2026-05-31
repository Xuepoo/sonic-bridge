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
    // Generate a 2.0 seconds wave with clean repeating onset pulses to trigger tempo estimation
    let sample_rate: u32 = 44100;
    let duration = 2.0f32;
    let num_samples = (duration * sample_rate as f32) as usize;
    let mut data = Vec::with_capacity(num_samples * 2);

    // Inject onset peaks every 0.4s to simulate ~150 BPM pulse
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let mut envelope = 1.0f32;
        // Pulse at 0.2s, 0.6s, 1.0s, 1.4s, 1.8s
        if (t - 0.2).abs() < 0.03
            || (t - 0.6).abs() < 0.03
            || (t - 1.0).abs() < 0.03
            || (t - 1.4).abs() < 0.03
            || (t - 1.8).abs() < 0.03
        {
            envelope = 12.0f32;
        }
        let val = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * envelope;
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
fn test_beat_synchronous_resampling_pipeline() {
    let wav_path = Path::new("tests/fixtures/mock_beat.wav");
    ensure_mock_wav(wav_path);

    let config = SonicConfig {
        beat_mode: true,
        ..Default::default()
    };

    let (meta, segs) = SonicPipeline::process_single(wav_path, &config).unwrap();

    assert!(meta.duration_seconds > 0.0);
    assert!(!segs.is_empty());

    // Output should register the dynamic tempo
    assert!(meta.estimated_bpm >= 60.0 && meta.estimated_bpm <= 180.0);

    // Check that segments are created and have valid description values
    for seg in &segs {
        assert!(!seg.time_range.is_empty());
        assert!(!seg.chord.is_empty());
        assert!(!seg.dynamic_level.is_empty());
        assert!(!seg.timbre_brightness.is_empty());
    }

    // Verify segments are merged correctly
    for i in 0..segs.len().saturating_sub(1) {
        let current = &segs[i];
        let next = &segs[i + 1];
        let is_identical = current.chord == next.chord
            && current.dynamic_level == next.dynamic_level
            && current.timbre_brightness == next.timbre_brightness
            && current.rhythm_activity == next.rhythm_activity;
        assert!(
            !is_identical,
            "Segment merger failed for beat mode: consecutive identical segments found!"
        );
    }
}
