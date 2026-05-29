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
