/// KeyDetector implements an ensemble-profile template matching algorithm.
/// It pre-computes 24 normalized key profiles (12 Major, 12 Minor) by averaging
/// Krumhansl-Kessler, Temperley, and Sha'ath profiles to form a unified, robust
/// template. It then detects the global tonal key using stable cosine similarity
/// and advanced musicological rule corrections.
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
    pub fn new() -> Self {
        let pitch_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];

        // 1. Krumhansl-Kessler experimental pitch class profiles
        let kk_major = vec![
            6.35, 2.23, 3.48, 1.50, 4.80, 4.09, 2.52, 5.19, 2.00, 3.66, 2.29, 2.88,
        ];
        let kk_minor = vec![
            6.33, 2.68, 3.52, 6.20, 1.50, 3.53, 2.54, 4.75, 4.80, 2.69, 3.34, 3.17,
        ];

        // 2. Temperley key profiles (academic/classical)
        let temp_major = vec![5.0, 2.0, 3.5, 2.0, 4.5, 4.0, 2.0, 4.5, 2.0, 3.5, 1.5, 4.0];
        let temp_minor = vec![5.0, 2.0, 3.5, 4.5, 2.0, 4.0, 2.0, 4.5, 3.5, 2.0, 1.5, 4.0];

        // 3. Sha'ath key profiles (modern/electronic)
        let shaath_major = vec![6.6, 2.0, 3.5, 2.3, 4.6, 4.0, 2.5, 5.2, 2.1, 3.7, 2.2, 3.0];
        let shaath_minor = vec![6.5, 2.6, 3.5, 6.0, 1.6, 3.5, 2.6, 4.8, 4.6, 2.6, 3.2, 3.1];

        // Compute averaged ensemble base profiles
        let mut base_major = [0.0f32; 12];
        let mut base_minor = [0.0f32; 12];
        for i in 0..12 {
            base_major[i] = (kk_major[i] + temp_major[i] + shaath_major[i]) / 3.0;
            base_minor[i] = (kk_minor[i] + temp_minor[i] + shaath_minor[i]) / 3.0;
        }

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
    /// - "Unknown" if input size is incorrect or similarity is too low.
    /// - The key name (e.g. "C Major", "A Minor", "D 宫调式") otherwise.
    pub fn detect(&self, chroma: &[f32]) -> String {
        if chroma.len() != 12 {
            return "Unknown".to_string();
        }

        let chroma_norm = chroma.iter().map(|&x| x * x).sum::<f32>().sqrt();

        if chroma_norm < 1e-4f32 {
            return "Silent".to_string();
        }

        let pitch_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];

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

        // Low confidence intercept to prevent G Major fallback bias in noisy / beatless environments
        if max_similarity < 0.35 {
            return "Unknown".to_string();
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
        if !is_major {
            let submediant_idx = (root_idx + 8) % 12;
            if chroma[submediant_idx] > 1.05 * chroma[root_idx] {
                final_root_idx = submediant_idx;
                final_is_major = true;
            } else {
                let flat_second_idx = (root_idx + 1) % 12;
                let second_idx = (root_idx + 2) % 12;
                if chroma[flat_second_idx] > 1.35 * chroma[second_idx] {
                    final_root_idx = submediant_idx;
                    final_is_major = true;
                }
            }
        }

        // Rule B: Enhanced Tonal Third Discriminator with 5th Harmonic Leakage Compensation
        let energy_minor_third = chroma[(final_root_idx + 3) % 12];
        let energy_major_third = chroma[(final_root_idx + 4) % 12];
        let root_energy = chroma[final_root_idx];

        // Subtract 5th harmonic leakage (approx 20% of root) from major third
        let corrected_major_third = (energy_major_third - 0.20 * root_energy).max(0.0);

        if final_is_major {
            if energy_minor_third > 0.88 * corrected_major_third {
                final_is_major = false;
            }
        } else {
            if corrected_major_third > 1.15 * energy_minor_third {
                final_is_major = true;
            }
        }

        let root_name = pitch_names[final_root_idx];

        // Rule C: Chinese Pentatonic Mode Mapping
        let root_energy_val = chroma[final_root_idx].max(1e-5);
        let gong_missing = chroma[(final_root_idx + 5) % 12] + chroma[(final_root_idx + 11) % 12];
        let shang_missing = chroma[(final_root_idx + 3) % 12] + chroma[(final_root_idx + 9) % 12];
        let jiao_missing = chroma[(final_root_idx + 1) % 12] + chroma[(final_root_idx + 7) % 12];
        let zhi_missing = chroma[(final_root_idx + 4) % 12] + chroma[(final_root_idx + 10) % 12];
        let yu_missing = chroma[(final_root_idx + 2) % 12] + chroma[(final_root_idx + 8) % 12];

        let pentatonic_threshold = 0.27 * root_energy_val;
        let mut best_pentatonic = None;
        let mut min_missing = f32::MAX;

        if gong_missing < pentatonic_threshold && gong_missing < min_missing {
            min_missing = gong_missing;
            best_pentatonic = Some("宫调式");
        }
        if shang_missing < pentatonic_threshold && shang_missing < min_missing {
            min_missing = shang_missing;
            best_pentatonic = Some("商调式");
        }
        if jiao_missing < pentatonic_threshold && jiao_missing < min_missing {
            min_missing = jiao_missing;
            best_pentatonic = Some("角调式");
        }
        if zhi_missing < pentatonic_threshold && zhi_missing < min_missing {
            min_missing = zhi_missing;
            best_pentatonic = Some("徵调式");
        }
        if yu_missing < pentatonic_threshold && yu_missing < min_missing {
            best_pentatonic = Some("羽调式");
        }

        if let Some(mode) = best_pentatonic {
            return format!("{} {}", root_name, mode);
        }

        if final_is_major {
            format!("{} Major", root_name)
        } else {
            format!("{} Minor", root_name)
        }
    }
}
