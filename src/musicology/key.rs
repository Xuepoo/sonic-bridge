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

        // Adjusted Krumhansl-Kessler key profiles to emphasize b3 and b6 for Minor, and b3 reduction for Major
        let base_major = vec![
            6.35, 2.23, 3.48, 1.50, 4.80, 4.09, 2.52, 5.19, 2.00, 3.66, 2.29, 2.88,
        ];
        let base_minor = vec![
            6.33, 2.68, 3.52, 6.20, 1.50, 3.53, 2.54, 4.75, 4.80, 2.69, 3.34, 3.17,
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

        println!("DEBUG CHROMA: {:?}", chroma);

        let pitch_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];

        let mut best_key = "Unknown".to_string();
        let mut max_similarity = -1.0f32;

        for (key_name, template) in &self.templates {
            let dot_product: f32 = chroma.iter().zip(template).map(|(&x, &y)| x * y).sum();
            let similarity = dot_product / chroma_norm;

            if similarity > 0.65 {
                println!("DEBUG SIM: {} -> {:.4}", key_name, similarity);
            }

            if similarity > max_similarity {
                max_similarity = similarity;
                best_key = key_name.clone();
            }
        }

        // Post-processing and correction based on Advanced Musicology Rules
        let parts: Vec<&str> = best_key.split_whitespace().collect();
        if parts.len() < 2 {
            return best_key;
        }
        let root_str = parts[0];
        let is_major = parts[1] == "Major";
        let root_idx = pitch_names.iter().position(|&r| r == root_str).unwrap_or(0);

        let mut final_root_idx = root_idx;
        let mut final_is_major = is_major;

        // Rule A: Mediant/Submediant Major Corrector (e.g. A Minor -> F Major)
        // If detected as X Minor, but the submediant Y = (X + 8) % 12 (F for A) has higher energy to X,
        // it is highly likely to be Y Major.
        if !is_major {
            let submediant_idx = (root_idx + 8) % 12;
            if chroma[submediant_idx] > 1.05 * chroma[root_idx] {
                final_root_idx = submediant_idx;
                final_is_major = true;
            } else {
                // Phrygian-second Mediant Corrector:
                // In X Minor, the second degree is (X + 2) % 12 (B for A), and the flat-second is (X + 1) % 12 (Bb for A).
                // If the flat-second is much stronger than the second degree, it means Bb is in key and B is out,
                // which strongly indicates Y Major ((X + 8) % 12 Major, i.e., F Major) where Bb is the perfect fourth.
                let flat_second_idx = (root_idx + 1) % 12;
                let second_idx = (root_idx + 2) % 12;
                if chroma[flat_second_idx] > 1.35 * chroma[second_idx] {
                    final_root_idx = submediant_idx;
                    final_is_major = true;
                }
            }
        }

        // Rule B: Tonal Third Discriminator (大小调硬核判定器)
        // Check the exact ratio of minor third (root + 3) vs major third (root + 4)
        let energy_minor_third = chroma[(final_root_idx + 3) % 12];
        let energy_major_third = chroma[(final_root_idx + 4) % 12];
        if final_is_major {
            if energy_minor_third > 0.88 * energy_major_third {
                final_is_major = false;
            }
        } else {
            if energy_major_third > 1.15 * energy_minor_third {
                final_is_major = true;
            }
        }

        // Rule C: Chinese Pentatonic Mode (宫/羽五声调式) Mapping
        // If Major, check Gong Mode (宫调式): lack of 4th and 7th degrees
        // If Minor, check Yu Mode (羽调式): lack of 2nd and 6th degrees
        let root_name = pitch_names[final_root_idx];
        if final_is_major {
            let fourth_idx = (final_root_idx + 5) % 12;
            let seventh_idx = (final_root_idx + 11) % 12;
            let root_energy = chroma[final_root_idx].max(1e-5);
            if (chroma[fourth_idx] + chroma[seventh_idx]) < 0.25 * root_energy {
                return format!("{} 宫调式", root_name);
            }
            format!("{} Major", root_name)
        } else {
            let second_idx = (final_root_idx + 2) % 12;
            let sixth_idx = (final_root_idx + 8) % 12;
            let root_energy = chroma[final_root_idx].max(1e-5);
            if (chroma[second_idx] + chroma[sixth_idx]) < 0.25 * root_energy {
                return format!("{} 羽调式", root_name);
            }
            format!("{} Minor", root_name)
        }
    }
}
