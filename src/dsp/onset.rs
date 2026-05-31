pub struct OnsetDetector {
    threshold: f32,
}

impl OnsetDetector {
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }

    /// Compute adaptive median-filtered Spectral Flux and Peak detection to locate structural boundaries
    pub fn detect_boundaries(&self, spectrogram: &[Vec<f32>]) -> Vec<usize> {
        let mut boundaries = Vec::new();
        if spectrogram.len() < 2 {
            return boundaries;
        }

        let num_frames = spectrogram.len();
        let num_bins = spectrogram[0].len();
        let mut fluxes = vec![0.0f32; num_frames];

        // 1. Calculate Spectral Flux
        for i in 1..num_frames {
            let mut flux = 0.0f32;
            for (&curr, &prev) in spectrogram[i].iter().zip(spectrogram[i - 1].iter()) {
                let diff = curr - prev;
                if diff > 0.0 {
                    flux += diff; // Capture positive intensity energy changes
                }
            }
            fluxes[i] = flux / num_bins as f32;
        }

        // 2. Sliding window median thresholding & peak detection
        let h = 5; // Window half-size
        let lambda = 1.5f32; // Median scaling factor
        let alpha = self.threshold * 0.05f32; // Offset factor bound to config

        let min_interval_frames = 15; // Approximately 348ms minimum interval to debounce dense transients
        let mut last_onset_frame = 0;

        for i in 1..num_frames {
            let current_flux = fluxes[i];

            let start = i.saturating_sub(h);
            let end = if i + h < num_frames {
                i + h
            } else {
                num_frames - 1
            };

            let mut window = fluxes[start..=end].to_vec();
            window.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let median = window[window.len() / 2];
            let adaptive_threshold = lambda * median + alpha;

            // Peak detection (local maximum constraints)
            let is_peak = current_flux >= fluxes[i - 1]
                && (i + 1 >= num_frames || current_flux >= fluxes[i + 1]);

            if current_flux > adaptive_threshold
                && is_peak
                && (last_onset_frame == 0 || i - last_onset_frame >= min_interval_frames)
            {
                boundaries.push(i);
                last_onset_frame = i;
            }
        }

        boundaries
    }
}
