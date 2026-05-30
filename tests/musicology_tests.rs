use sonic_bridge::musicology::chroma::ChordClassifier;

#[test]
fn test_chord_classification_c_major() {
    let classifier = ChordClassifier::new();

    // Chroma vector representing C Major (C, E, G activated)
    let mut chroma_c_major = vec![0.0f32; 12];
    chroma_c_major[0] = 1.0; // C
    chroma_c_major[4] = 0.8; // E
    chroma_c_major[7] = 0.9; // G

    let classified = classifier.classify(&chroma_c_major);
    assert_eq!(classified, "C");
}

#[test]
fn test_chord_classification_silent() {
    let classifier = ChordClassifier::new();

    // All zero chroma vector
    let chroma_silent = vec![0.0f32; 12];
    let classified = classifier.classify(&chroma_silent);
    assert_eq!(classified, "Silent");

    // Very low energy chroma vector
    let chroma_low = vec![0.00001f32; 12];
    let classified_low = classifier.classify(&chroma_low);
    assert_eq!(classified_low, "Silent");
}

#[test]
fn test_chord_classification_unknown() {
    let classifier = ChordClassifier::new();

    // A completely flat chroma vector with non-zero energy
    // This should result in poor similarity match with any specific chord
    let chroma_flat = vec![1.0f32; 12];
    let classified_flat = classifier.classify(&chroma_flat);
    assert_eq!(classified_flat, "Unknown");
}

#[test]
fn test_advanced_chords_classification() {
    let classifier = ChordClassifier::new();

    // 1. Cmaj7 (C: 0, E: 4, G: 7, B: 11)
    let mut chroma_c_maj7 = vec![0.0f32; 12];
    chroma_c_maj7[0] = 1.0;
    chroma_c_maj7[4] = 0.9;
    chroma_c_maj7[7] = 0.8;
    chroma_c_maj7[11] = 0.85;
    assert_eq!(classifier.classify(&chroma_c_maj7), "Cmaj7");

    // 2. Am7 (A: 9, C: 0, E: 4, G: 7)
    let mut chroma_a_min7 = vec![0.0f32; 12];
    chroma_a_min7[9] = 1.0;
    chroma_a_min7[0] = 0.85;
    chroma_a_min7[4] = 0.8;
    chroma_a_min7[7] = 0.9;
    assert_eq!(classifier.classify(&chroma_a_min7), "Am7");

    // 3. C7 (C: 0, E: 4, G: 7, A#/Bb: 10)
    let mut chroma_c_dom7 = vec![0.0f32; 12];
    chroma_c_dom7[0] = 1.0;
    chroma_c_dom7[4] = 0.85;
    chroma_c_dom7[7] = 0.8;
    chroma_c_dom7[10] = 0.9;
    assert_eq!(classifier.classify(&chroma_c_dom7), "C7");

    // 4. Csus4 (C: 0, F: 5, G: 7)
    let mut chroma_c_sus4 = vec![0.0f32; 12];
    chroma_c_sus4[0] = 1.0;
    chroma_c_sus4[5] = 0.95;
    chroma_c_sus4[7] = 0.85;
    assert_eq!(classifier.classify(&chroma_c_sus4), "Csus4");
}

#[test]
fn test_global_key_detection() {
    use sonic_bridge::musicology::key::KeyDetector;
    let detector = KeyDetector::new();

    // 1. C Major biased profile: C, D, E, F, G, A, B notes are dominant, C# etc are close to zero
    // K-S C Major template matches this closely
    let c_major_profile = vec![6.0, 0.1, 3.0, 0.1, 4.0, 4.0, 0.1, 5.0, 0.1, 3.5, 0.1, 2.5];
    let key = detector.detect(&c_major_profile);
    assert_eq!(key, "C Major");

    // 2. A Minor biased profile
    // Standard A minor: A (9), B (11), C (0), D (2), E (4), F (5), G (7)
    // Relative shift of minor template to A (root = 9)
    let mut a_minor_profile = vec![0.0f32; 12];
    let base_minor_profile = vec![
        6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
    ];
    // Rotate minor template by 9 places to simulate A Minor profile
    for i in 0..12 {
        a_minor_profile[(i + 9) % 12] = base_minor_profile[i];
    }
    let key_min = detector.detect(&a_minor_profile);
    assert_eq!(key_min, "A Minor");
}
