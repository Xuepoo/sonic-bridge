/// KeyDetector implements the Krumhansl-Schmuckler key profiles templates matching algorithm.
/// It pre-computes 24 normalized key profiles (12 Major, 12 Minor) and detects the global tonal key
/// of a given cumulative chroma centroid vector with high DSP performance.
pub struct KeyDetector {
    templates: Vec<(String, Vec<f32>)>,
}

impl Default for KeyDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyDetector {
    /// Creates a new `KeyDetector` and pre-computes 24 key templates.
    /// Uses Krumhansl-Kessler experimental pitch class profiles.
    pub fn new() -> Self {
        let pitch_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];

        // Krumhansl-Kessler key profiles
        let base_major = vec![
            6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
        ];
        let base_minor = vec![
            6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
        ];

        let mut templates = Vec::with_capacity(24);

        for i in 0..12 {
            let root = pitch_names[i].to_string();

            // 1. Shift and normalize Major Profile
            let mut major_shifted = [0.0f32; 12];
            for j in 0..12 {
                major_shifted[j] = base_major[(j + 12 - i) % 12];
            }
            let major_norm = major_shifted.iter().map(|&x| x * x).sum::<f32>().sqrt();
            let major_normalized = major_shifted
                .iter()
                .map(|&x| x / major_norm)
                .collect::<Vec<f32>>();
            templates.push((format!("{} Major", root), major_normalized));

            // 2. Shift and normalize Minor Profile
            let mut minor_shifted = [0.0f32; 12];
            for j in 0..12 {
                minor_shifted[j] = base_minor[(j + 12 - i) % 12];
            }
            let minor_norm = minor_shifted.iter().map(|&x| x * x).sum::<f32>().sqrt();
            let minor_normalized = minor_shifted
                .iter()
                .map(|&x| x / minor_norm)
                .collect::<Vec<f32>>();
            templates.push((format!("{} Minor", root), minor_normalized));
        }

        Self { templates }
    }

    /// Detects the global tonal key based on a cumulative chroma centroid vector.
    ///
    /// # Returns
    /// - "Silent" if the vector energy is too low.
    /// - "Unknown" if input size is incorrect.
    /// - The key name (e.g. "C Major", "A Minor") otherwise.
    pub fn detect(&self, chroma: &[f32]) -> String {
        if chroma.len() != 12 {
            return "Unknown".to_string();
        }

        let chroma_norm = chroma.iter().map(|&x| x * x).sum::<f32>().sqrt();

        if chroma_norm < 1e-4f32 {
            return "Silent".to_string();
        }

        let mut best_key = "Unknown".to_string();
        let mut max_similarity = -1.0f32;

        for (key_name, template) in &self.templates {
            let dot_product: f32 = chroma.iter().zip(template).map(|(&x, &y)| x * y).sum();
            let similarity = dot_product / chroma_norm;

            if similarity > max_similarity {
                max_similarity = similarity;
                best_key = key_name.clone();
            }
        }

        best_key
    }
}
