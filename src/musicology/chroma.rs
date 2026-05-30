/// ChordClassifier pre-generates 12-dimensional normalized templates for 24 major and minor chords
/// and classifies chroma vectors using Cosine Similarity to achieve high DSP performance.
pub struct ChordClassifier {
    templates: Vec<(String, Vec<f32>)>,
}

impl Default for ChordClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ChordClassifier {
    /// Creates a new `ChordClassifier` and pre-generates 24 chord templates (12 Major, 12 Minor).
    /// Each template is a normalized 12-dimensional vector.
    pub fn new() -> Self {
        let pitch_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let mut templates = Vec::with_capacity(84);

        // Pre-compute normalized values for active notes (3 or 4 notes)
        let val3 = 1.0f32 / 3.0f32.sqrt();
        let val4 = 1.0f32 / 4.0f32.sqrt();

        for i in 0..12 {
            let root = pitch_names[i].to_string();

            // 1. Major chord: root, root+4, root+7
            let mut major_template = vec![0.0f32; 12];
            major_template[i] = val3;
            major_template[(i + 4) % 12] = val3;
            major_template[(i + 7) % 12] = val3;
            templates.push((root.clone(), major_template));

            // 2. Minor chord: root, root+3, root+7
            let mut minor_template = vec![0.0f32; 12];
            minor_template[i] = val3;
            minor_template[(i + 3) % 12] = val3;
            minor_template[(i + 7) % 12] = val3;
            templates.push((format!("{}m", root), minor_template));

            // 3. Major 7th chord: root, root+4, root+7, root+11
            let mut maj7_template = vec![0.0f32; 12];
            maj7_template[i] = val4;
            maj7_template[(i + 4) % 12] = val4;
            maj7_template[(i + 7) % 12] = val4;
            maj7_template[(i + 11) % 12] = val4;
            templates.push((format!("{}maj7", root), maj7_template));

            // 4. Minor 7th chord: root, root+3, root+7, root+10
            let mut min7_template = vec![0.0f32; 12];
            min7_template[i] = val4;
            min7_template[(i + 3) % 12] = val4;
            min7_template[(i + 7) % 12] = val4;
            min7_template[(i + 10) % 12] = val4;
            templates.push((format!("{}m7", root), min7_template));

            // 5. Dominant 7th chord: root, root+4, root+7, root+10
            let mut dom7_template = vec![0.0f32; 12];
            dom7_template[i] = val4;
            dom7_template[(i + 4) % 12] = val4;
            dom7_template[(i + 7) % 12] = val4;
            dom7_template[(i + 10) % 12] = val4;
            templates.push((format!("{}7", root), dom7_template));

            // 6. sus2 chord: root, root+2, root+7
            let mut sus2_template = vec![0.0f32; 12];
            sus2_template[i] = val3;
            sus2_template[(i + 2) % 12] = val3;
            sus2_template[(i + 7) % 12] = val3;
            templates.push((format!("{}sus2", root), sus2_template));

            // 7. sus4 chord: root, root+5, root+7
            let mut sus4_template = vec![0.0f32; 12];
            sus4_template[i] = val3;
            sus4_template[(i + 5) % 12] = val3;
            sus4_template[(i + 7) % 12] = val3;
            templates.push((format!("{}sus4", root), sus4_template));
        }

        Self { templates }
    }

    /// Classifies the input 12-dimensional chroma vector.
    ///
    /// # Performance
    /// Complexity: O(1) time and space complexity since the dimensions are fixed (12 elements, 24 templates).
    /// Highly optimized for DSP pipelines by pre-normalizing the templates.
    ///
    /// # Returns
    /// - "Silent" if the vector energy is too low.
    /// - "Unknown" if the cosine similarity score is poor (< 0.6).
    /// - The chord name (e.g. "C", "Am") otherwise.
    pub fn classify(&self, chroma: &[f32]) -> String {
        if chroma.len() != 12 {
            return "Unknown".to_string();
        }

        // Calculate L2 norm of the input chroma vector
        let chroma_norm = chroma.iter().map(|&x| x * x).sum::<f32>().sqrt();

        // If the energy is too low or all zeros, return "Silent"
        if chroma_norm < 1e-4f32 {
            return "Silent".to_string();
        }

        let mut best_chord = "Unknown".to_string();
        let mut max_similarity = -1.0f32;

        for (chord_name, template) in &self.templates {
            // Since template is already normalized, dot product divided by chroma_norm is the cosine similarity
            let dot_product: f32 = chroma.iter().zip(template).map(|(&x, &y)| x * y).sum();
            let similarity = dot_product / chroma_norm;

            if similarity > max_similarity {
                max_similarity = similarity;
                best_chord = chord_name.clone();
            }
        }

        // If matching score is poor, return "Unknown"
        if max_similarity < 0.6f32 {
            "Unknown".to_string()
        } else {
            best_chord
        }
    }
}
