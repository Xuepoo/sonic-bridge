use sonic_bridge::dsp::style::StyleClassifier;
use sonic_bridge::pipeline::{EnsembleSelector, PrecomputedFeatures};

#[test]
fn test_ensemble_selector_c_major() {
    let selector = EnsembleSelector::new();

    // Chroma representing standard Western C Major
    let mut chroma = [0.0f32; 12];
    chroma[0] = 6.0; // C
    chroma[2] = 3.0; // D
    chroma[4] = 4.0; // E
    chroma[5] = 4.0; // F
    chroma[7] = 5.0; // G
    chroma[9] = 3.5; // A
    chroma[11] = 2.5; // B

    let features = PrecomputedFeatures {
        envelope: vec![1.0; 100],
        diff_variance: 0.12,
        onset_density: 1.2,
        smooth_hist: [1.0; 41],
        frame_duration: 0.023,
        max_confidence: 0.6,
        best_lag: 40,
        corr_norm: vec![0.5; 100],
        variance: 0.1,
        peak_coeff: 0.5,
    };

    let style = StyleClassifier::classify(
        &vec![vec![1.0; 513]; 100],
        &chroma,
        features.onset_density,
        features.diff_variance,
        features.max_confidence,
        22050.0,
    );

    // Verify style vector: should prioritize electronic_pop or classical
    assert!(style.electronic_pop > 0.40 || style.classical > 0.40);

    let (bpm, key, conf) = selector.select(&chroma, &vec![vec![1.0; 513]; 100], &style, &features);

    assert!(bpm > 0.0);
    assert_eq!(key, "C Major");
    assert!(conf > 0.1);
}

#[test]
fn test_ensemble_selector_chinese_pentatonic() {
    let selector = EnsembleSelector::new();

    // Chroma representing D Pentatonic Gong mode (D, E, F#, A, B active; missing G (5) and C# (1))
    let mut chroma = [0.0f32; 12];
    chroma[2] = 6.0; // D (宫)
    chroma[4] = 4.5; // E (商)
    chroma[6] = 5.0; // F# (角)
    chroma[7] = 0.01; // G is missing (perfect fourth)
    chroma[9] = 5.5; // A (徵)
    chroma[11] = 4.0; // B (羽)
    chroma[1] = 0.01; // C# is missing (major seventh)

    let features = PrecomputedFeatures {
        envelope: vec![1.0; 100],
        diff_variance: 0.02,
        onset_density: 0.5,
        smooth_hist: [1.0; 41],
        frame_duration: 0.023,
        max_confidence: 0.5,
        best_lag: 40,
        corr_norm: vec![0.5; 100],
        variance: 0.1,
        peak_coeff: 0.5,
    };

    let style = StyleClassifier::classify(
        &vec![vec![1.0; 513]; 100],
        &chroma,
        features.onset_density,
        features.diff_variance,
        features.max_confidence,
        22050.0,
    );

    // Verify style vector: should prioritize traditional_chinese
    assert!(style.traditional_chinese > 0.50);

    let (_, key, conf) = selector.select(&chroma, &vec![vec![1.0; 513]; 100], &style, &features);
    assert_eq!(key, "D 宫调式");
    assert!(conf > 0.1);
}
