use crate::dsp::mfcc::MfccEngine;

/// Representation of music style probabilities derived from Timbre (MFCC) and temporal features.
#[derive(Debug, Clone, Default)]
pub struct StyleVector {
    pub classical: f32,
    pub electronic_pop: f32,
    pub traditional_chinese: f32,
    pub jazz_rubato: f32,
    pub ambient_free: f32,
    pub triple_meter: bool,
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

        // 1.5. OPIH Time-Signature Analysis (Autocorrelation on Spectral Flux)
        let limit_frames = spectrogram.len().min(1300);
        let mut triple_meter = false;
        if limit_frames > 200 {
            let mut fluxes = vec![0.0f32; limit_frames];
            let num_bins = spectrogram[0].len();
            for i in 1..limit_frames {
                let mut flux = 0.0f32;
                for (&curr, &prev) in spectrogram[i].iter().zip(spectrogram[i - 1].iter()) {
                    let diff = curr - prev;
                    if diff > 0.0 {
                        flux += diff;
                    }
                }
                fluxes[i] = flux / num_bins as f32;
            }

            // Autocorrelation over lag range [10, 100] (roughly 130ms to 2.3s intervals)
            let min_lag = 10;
            let max_lag = 100.min(limit_frames / 2);
            let mut acf = vec![0.0f32; max_lag - min_lag];
            for lag in min_lag..max_lag {
                let mut sum = 0.0f32;
                for n in lag..limit_frames {
                    sum += fluxes[n] * fluxes[n - lag];
                }
                acf[lag - min_lag] = sum;
            }

            // Peak extraction
            let mut peaks = Vec::new();
            for i in 1..(acf.len() - 1) {
                if acf[i] > acf[i - 1] && acf[i] > acf[i + 1] && acf[i] > 1e-3 {
                    peaks.push((i + min_lag, acf[i]));
                }
            }
            peaks.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            if peaks.len() >= 2 {
                let p1 = peaks[0].0 as f32;
                let p2 = peaks[1].0 as f32;
                let ratio = p2 / p1;
                // Triple/odd meter ratio check: 3/4 or 6/8 lag patterns (ratios of 1.5, 3.0, 0.67, 0.75)
                if (ratio - 1.5).abs() < 0.1
                    || (ratio - 3.0).abs() < 0.15
                    || (ratio - 0.75).abs() < 0.08
                    || (ratio - 0.67).abs() < 0.08
                {
                    triple_meter = true;
                }
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

        if triple_meter {
            jazz_rubato += 0.40;
        }

        // Normalize style probabilities
        let sum = classical + electronic_pop + traditional_chinese + jazz_rubato + ambient_free;
        StyleVector {
            classical: classical / sum,
            electronic_pop: electronic_pop / sum,
            traditional_chinese: traditional_chinese / sum,
            jazz_rubato: jazz_rubato / sum,
            ambient_free: ambient_free / sum,
            triple_meter,
        }
    }
}
