use sonic_bridge::dsp::mfcc::MfccEngine;

#[test]
fn test_mfcc_initialization_and_dimensions() {
    let num_mel_filters = 26;
    let num_coefficients = 13;
    let sample_rate = 22050.0f32;
    let fft_size = 1024;

    let engine = MfccEngine::new(num_mel_filters, num_coefficients, sample_rate, fft_size);

    // Generate a dummy single frame magnitude spectrum (size: 513)
    let magnitude_spectrum = vec![1.0f32; 513];
    let coefs = engine.compute_frame(&magnitude_spectrum);

    // Verify dimension
    assert_eq!(
        coefs.len(),
        13,
        "MFCC output must contain exactly 13 coefficients"
    );

    // Verify all values are valid floats (no NaN or Inf)
    for (i, &val) in coefs.iter().enumerate() {
        assert!(
            val.is_finite(),
            "Coefficient at index {} must be finite, got {}",
            i,
            val
        );
    }
}

#[test]
fn test_mfcc_frequency_discrimination() {
    let engine = MfccEngine::new(26, 13, 22050.0, 1024);

    // Frame A: Strong low frequency energy (e.g., concentrated around 100 Hz, which maps to low bins)
    // 100 Hz / (22050 / 1024) ≈ bin index 5
    let mut low_freq_spectrum = vec![0.0f32; 513];
    for i in 0..10 {
        low_freq_spectrum[i] = 10.0;
    }

    // Frame B: Strong high frequency energy (e.g., concentrated around 8000 Hz, which maps to high bins)
    // 8000 Hz / (22050 / 1024) ≈ bin index 371
    let mut high_freq_spectrum = vec![0.0f32; 513];
    for i in 360..380 {
        high_freq_spectrum[i] = 10.0;
    }

    let coefs_low = engine.compute_frame(&low_freq_spectrum);
    let coefs_high = engine.compute_frame(&high_freq_spectrum);

    // Since low frequency maps to low Mel bands and high frequency to high Mel bands,
    // their overall MFCC distributions must differ.
    let mut difference_sum = 0.0f32;
    for (a, b) in coefs_low.iter().zip(coefs_high.iter()) {
        difference_sum += (a - b).abs();
    }

    assert!(
        difference_sum > 1.0,
        "MFCC engine should clearly distinguish between low and high frequencies (diff sum: {})",
        difference_sum
    );
}

#[test]
fn test_mfcc_spectrogram_processing() {
    let engine = MfccEngine::new(26, 13, 22050.0, 1024);

    // Create a 2D spectrogram with 5 frames
    let spectrogram = vec![vec![1.0f32; 513]; 5];
    let mfccs = engine.compute(&spectrogram);

    assert_eq!(mfccs.len(), 5, "Should output exactly 5 frames");
    for frame in &mfccs {
        assert_eq!(frame.len(), 13, "Each frame must have 13 coefficients");
    }
}
