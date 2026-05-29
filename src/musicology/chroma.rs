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
        let mut templates = Vec::with_capacity(24);

        // Pre-compute 1.0 / sqrt(3) as the normalized value for active notes
        let val = 1.0f32 / 3.0f32.sqrt();

        for i in 0..12 {
            // Major chord: root, root+4, root+7
            let mut major_template = vec![0.0f32; 12];
            major_template[i] = val;
            major_template[(i + 4) % 12] = val;
            major_template[(i + 7) % 12] = val;
            templates.push((pitch_names[i].to_string(), major_template));

            // Minor chord: root, root+3, root+7
            let mut minor_template = vec![0.0f32; 12];
            minor_template[i] = val;
            minor_template[(i + 3) % 12] = val;
            minor_template[(i + 7) % 12] = val;
            templates.push((format!("{}m", pitch_names[i]), minor_template));
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
