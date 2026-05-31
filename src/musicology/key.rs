/// KeyDetector implements an ensemble template matching algorithm combining
/// Krumhansl-Kessler, Temperley, and Sha'ath key profiles.
/// It pre-computes 24 key templates across all three profiles and runs
/// a two-pass root-locking and major/minor/pentatonic mode classifier.
pub struct KeyDetector {
    // 24 templates: each entry contains:
    // (KeyName, KKProfile, TemperleyProfile, ShaathProfile)
    templates: Vec<(String, Vec<f32>, Vec<f32>, Vec<f32>)>,
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

        let mut templates = Vec::with_capacity(24);

        for i in 0..12 {
            let root = pitch_names[i].to_string();

            // Shift and normalize Major profiles
            let mut kk_maj_shifted = [0.0f32; 12];
            let mut temp_maj_shifted = [0.0f32; 12];
            let mut shaath_maj_shifted = [0.0f32; 12];
            for j in 0..12 {
                kk_maj_shifted[j] = kk_major[(j + 12 - i) % 12];
                temp_maj_shifted[j] = temp_major[(j + 12 - i) % 12];
                shaath_maj_shifted[j] = shaath_major[(j + 12 - i) % 12];
            }
            let kk_maj_norm = kk_maj_shifted.iter().map(|&x| x * x).sum::<f32>().sqrt();
            let temp_maj_norm = temp_maj_shifted.iter().map(|&x| x * x).sum::<f32>().sqrt();
            let shaath_maj_norm = shaath_maj_shifted
                .iter()
                .map(|&x| x * x)
                .sum::<f32>()
                .sqrt();

            let kk_maj_normalized = kk_maj_shifted
                .iter()
                .map(|&x| x / kk_maj_norm)
                .collect::<Vec<f32>>();
            let temp_maj_normalized = temp_maj_shifted
                .iter()
                .map(|&x| x / temp_maj_norm)
                .collect::<Vec<f32>>();
            let shaath_maj_normalized = shaath_maj_shifted
                .iter()
                .map(|&x| x / shaath_maj_norm)
                .collect::<Vec<f32>>();

            templates.push((
                format!("{} Major", root),
                kk_maj_normalized,
                temp_maj_normalized,
                shaath_maj_normalized,
            ));

            // Shift and normalize Minor profiles
            let mut kk_min_shifted = [0.0f32; 12];
            let mut temp_min_shifted = [0.0f32; 12];
            let mut shaath_min_shifted = [0.0f32; 12];
            for j in 0..12 {
                kk_min_shifted[j] = kk_minor[(j + 12 - i) % 12];
                temp_min_shifted[j] = temp_minor[(j + 12 - i) % 12];
                shaath_min_shifted[j] = shaath_minor[(j + 12 - i) % 12];
            }
            let kk_min_norm = kk_min_shifted.iter().map(|&x| x * x).sum::<f32>().sqrt();
            let temp_min_norm = temp_min_shifted.iter().map(|&x| x * x).sum::<f32>().sqrt();
            let shaath_min_norm = shaath_min_shifted
                .iter()
                .map(|&x| x * x)
                .sum::<f32>()
                .sqrt();

            let kk_min_normalized = kk_min_shifted
                .iter()
                .map(|&x| x / kk_min_norm)
                .collect::<Vec<f32>>();
            let temp_min_normalized = temp_min_shifted
                .iter()
                .map(|&x| x / temp_min_norm)
                .collect::<Vec<f32>>();
            let shaath_min_normalized = shaath_min_shifted
                .iter()
                .map(|&x| x / shaath_min_norm)
                .collect::<Vec<f32>>();

            templates.push((
                format!("{} Minor", root),
                kk_min_normalized,
                temp_min_normalized,
                shaath_min_normalized,
            ));
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

        println!("DEBUG CHROMA: {:?}", chroma);

        let pitch_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];

        let mut best_root_idx = 0;
        let mut max_root_score = -1e9f32;
        let mut max_overall_similarity = -1.0f32;

        // Pass 1: Lock tonal root using the ensemble profile sum
        for i in 0..12 {
            let maj_template = &self.templates[i * 2];
            let min_template = &self.templates[i * 2 + 1];

            // Major similarities
            let maj_kk_dot: f32 = chroma
                .iter()
                .zip(&maj_template.1)
                .map(|(&x, &y)| x * y)
                .sum();
            let maj_temp_dot: f32 = chroma
                .iter()
                .zip(&maj_template.2)
                .map(|(&x, &y)| x * y)
                .sum();
            let maj_shaath_dot: f32 = chroma
                .iter()
                .zip(&maj_template.3)
                .map(|(&x, &y)| x * y)
                .sum();
            let maj_sim = (maj_kk_dot + maj_temp_dot + maj_shaath_dot) / (3.0 * chroma_norm);

            // Minor similarities
            let min_kk_dot: f32 = chroma
                .iter()
                .zip(&min_template.1)
                .map(|(&x, &y)| x * y)
                .sum();
            let min_temp_dot: f32 = chroma
                .iter()
                .zip(&min_template.2)
                .map(|(&x, &y)| x * y)
                .sum();
            let min_shaath_dot: f32 = chroma
                .iter()
                .zip(&min_template.3)
                .map(|(&x, &y)| x * y)
                .sum();
            let min_sim = (min_kk_dot + min_temp_dot + min_shaath_dot) / (3.0 * chroma_norm);

            let root_score = maj_sim + min_sim;

            if maj_sim > max_overall_similarity {
                max_overall_similarity = maj_sim;
            }
            if min_sim > max_overall_similarity {
                max_overall_similarity = min_sim;
            }

            if root_score > max_root_score {
                max_root_score = root_score;
                best_root_idx = i;
            }
        }

        // Low confidence intercept to prevent G Major fallback bias in noisy / beatless environments
        if max_overall_similarity < 0.35 {
            return "Unknown".to_string();
        }

        let root_name = pitch_names[best_root_idx];

        // Pass 2: Chinese Pentatonic Mode Mapping
        let root_energy = chroma[best_root_idx].max(1e-5);
        let gong_missing = chroma[(best_root_idx + 5) % 12] + chroma[(best_root_idx + 11) % 12];
        let shang_missing = chroma[(best_root_idx + 3) % 12] + chroma[(best_root_idx + 9) % 12];
        let jiao_missing = chroma[(best_root_idx + 1) % 12] + chroma[(best_root_idx + 7) % 12];
        let zhi_missing = chroma[(best_root_idx + 4) % 12] + chroma[(best_root_idx + 10) % 12];
        let yu_missing = chroma[(best_root_idx + 2) % 12] + chroma[(best_root_idx + 8) % 12];

        let pentatonic_threshold = 0.25 * root_energy;
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

        // Pass 3: Tonal Third & Sixth Discriminator for Major vs Minor
        let energy_minor = chroma[(best_root_idx + 3) % 12] + chroma[(best_root_idx + 8) % 12];
        let energy_major = chroma[(best_root_idx + 4) % 12] + chroma[(best_root_idx + 9) % 12];

        let is_minor = energy_minor > 0.90 * energy_major;

        if is_minor {
            format!("{} Minor", root_name)
        } else {
            format!("{} Major", root_name)
        }
    }
}
