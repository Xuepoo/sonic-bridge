use sonic_bridge::dsp::onset::OnsetDetector;

#[test]
fn test_onset_triggered_segmentation() {
    // Generate spectrogram representing a sudden transient clap at frame 10
    let mut spectrogram = vec![vec![0.1f32; 513]; 20];
    spectrogram[10] = vec![2.5f32; 513]; // peak clap

    let detector = OnsetDetector::new(0.5); // Threshold 0.5
    let boundary_frames = detector.detect_boundaries(&spectrogram);

    // It should detect a boundary right at frame 10
    assert!(boundary_frames.contains(&10));
}
