use crate::dsp::mfcc::MfccEngine;

/// Representation of music style probabilities derived from Timbre (MFCC) and temporal features.
#[derive(Debug, Clone, Default)]
pub struct StyleVector {
    pub classical: f32,
    pub electronic_pop: f32,
    pub traditional_chinese: f32,
    pub jazz_rubato: f32,
    pub ambient_free: f32,
}

pub struct StyleClassifier;

impl StyleClassifier {
    /// Classifies the overall music style profile using timbre features (MFCC) and temporal cues.
    pub fn classify(
        spectrogram: &[Vec<f32>],
        chroma: &[f32; 12],
        onset_density: f32,
        diff_variance: f32,
        max_confidence: f32,
        sample_rate: f32,
    ) -> StyleVector {
        // 1. Calculate average MFCCs across frames to capture timbre
        let mfcc_engine = MfccEngine::new(26, 13, sample_rate, 1024);
        let mut avg_mfccs = [0.0f32; 13];
        let mut frame_count = 0;

        // Downsample spectrogram frames for compute efficiency (step by 4)
        for i in (0..spectrogram.len()).step_by(4) {
            let frame = &spectrogram[i];
            let mfccs = mfcc_engine.compute_frame(frame);
            for (j, val) in avg_mfccs.iter_mut().enumerate() {
                *val += mfccs[j];
            }
            frame_count += 1;
        }

        if frame_count > 0 {
            for val in &mut avg_mfccs {
                *val /= frame_count as f32;
            }
        }

        // 2. Identify ambient/free rhythm characteristics:
        // - Very low onset density (< 0.22)
        // - Extremely low envelope difference variance (< 0.018)
        let is_ambient = onset_density < 0.22 && diff_variance < 0.018;

        // 3. Identify traditional Chinese pentatonic traits:
        // Chinese pentatonic scales focus their energy within exactly 5 pitches (宫商角徵羽),
        // meaning the top 5 chroma bins hold a massive proportion of the total chroma energy.
        let mut sorted_chroma = chroma.to_vec();
        sorted_chroma.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        let sum_chroma: f32 = chroma.iter().sum();
        let top5_sum: f32 = sorted_chroma.iter().take(5).sum();
        let pentatonic_ratio = if sum_chroma > 1e-4 {
            top5_sum / sum_chroma
        } else {
            0.0
        };

        // If top 5 bins have >81% energy, it's highly pentatonic
        let is_chinese_folk = pentatonic_ratio > 0.81;

        // 4. Establish heuristic weights
        let mut classical = 0.05f32;
        let mut electronic_pop = 0.05f32;
        let mut traditional_chinese = 0.05f32;
        let mut jazz_rubato = 0.05f32;
        let mut ambient_free = 0.05f32;

        if is_ambient {
            ambient_free += 0.85;
            classical += 0.10;
        } else if is_chinese_folk {
            traditional_chinese += 0.80;
            classical += 0.10;
            electronic_pop += 0.05;
        } else if diff_variance >= 0.09 && avg_mfccs[1] < 4.0 {
            electronic_pop += 0.80;
            jazz_rubato += 0.10;
        } else {
            // Acoustic Branch: Differentiate classical solo vs syncopated jazz standards
            if onset_density > 0.30 && max_confidence >= 0.35 {
                jazz_rubato += 0.75;
                classical += 0.15;
            } else {
                classical += 0.75;
                jazz_rubato += 0.15;
            }
        }

        // Normalize style probabilities
        let sum = classical + electronic_pop + traditional_chinese + jazz_rubato + ambient_free;
        StyleVector {
            classical: classical / sum,
            electronic_pop: electronic_pop / sum,
            traditional_chinese: traditional_chinese / sum,
            jazz_rubato: jazz_rubato / sum,
            ambient_free: ambient_free / sum,
        }
    }
}
