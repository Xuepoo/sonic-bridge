pub struct OnsetDetector {
    threshold: f32,
}

impl OnsetDetector {
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }

    /// Compute Spectral Flux (difference between consecutive frames) to locate structural boundaries
    pub fn detect_boundaries(&self, spectrogram: &[Vec<f32>]) -> Vec<usize> {
        let mut boundaries = Vec::new();
        if spectrogram.len() < 2 {
            return boundaries;
        }

        let num_bins = spectrogram[0].len();

        for i in 1..spectrogram.len() {
            let mut flux = 0.0f32;
            for (&curr, &prev) in spectrogram[i].iter().zip(spectrogram[i - 1].iter()) {
                let diff = curr - prev;
                if diff > 0.0 {
                    flux += diff; // Capture positive intensity energy changes
                }
            }

            // Normalize flux by frequency bins
            let norm_flux = flux / num_bins as f32;
            if norm_flux > self.threshold {
                boundaries.push(i);
            }
        }

        boundaries
    }
}
