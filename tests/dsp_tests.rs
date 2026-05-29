use sonic_bridge::dsp::spectrogram::StftEngine;

#[test]
fn test_stft_processing() {
    let sample_rate = 22050.0f32;
    let mut signal = vec![0.0f32; 22050]; // 1 second signal
                                          // Generate simple 440Hz sine wave
    for i in 0..22050 {
        signal[i] = (2.0 * std::f32::consts::PI * 440.0 * (i as f32) / sample_rate).sin();
    }

    let engine = StftEngine::new(1024, 512);
    let spectrogram = engine.compute(&signal).unwrap();

    // Spectrogram should have multiple frames, each with 513 frequency bins
    assert!(!spectrogram.is_empty(), "Spectrogram should not be empty");
    for frame in &spectrogram {
        assert_eq!(
            frame.len(),
            513,
            "Each frame should have exactly 513 frequency bins"
        );
    }

    // Mathematical correctness verification:
    // With 22050Hz sample rate and 1024 window size, bin size is 22050 / 1024 = 21.533Hz.
    // 440Hz should fall into bin index around: 440.0 / 21.533 = 20.43 -> bin 20 or 21.
    // We expect the peak energy in a mid-signal frame (e.g., frame 10) to be around bin 20.
    let mid_frame = &spectrogram[10];
    let mut max_val = -1.0f32;
    let mut max_idx = 0;
    for (idx, &val) in mid_frame.iter().enumerate() {
        if val > max_val {
            max_val = val;
            max_idx = idx;
        }
    }

    // The peak should be exactly at bin 20 or 21
    assert!(
        max_idx == 20 || max_idx == 21,
        "Peak frequency bin should be 20 or 21 (actual: {})",
        max_idx
    );

    // All magnitudes should be non-negative
    for frame in &spectrogram {
        for &magnitude in frame {
            assert!(magnitude >= 0.0, "Magnitude must be non-negative");
        }
    }
}
