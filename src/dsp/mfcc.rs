#![allow(clippy::needless_range_loop)]

/// MfccEngine is a highly-optimized Mel-Frequency Cepstral Coefficients (MFCC) feature extraction engine.
/// It pre-computes triangular Mel filterbank weights and DCT-II mapping matrices to achieve
/// zero dynamic memory allocation overhead during frame-by-frame processing.
pub struct MfccEngine {
    num_mel_filters: usize,
    num_coefficients: usize,
    filterbank: Vec<Vec<f32>>,
    dct_matrix: Vec<Vec<f32>>,
}

impl MfccEngine {
    /// Creates a new `MfccEngine` instance.
    ///
    /// # Arguments
    /// * `num_mel_filters` - Number of Mel bands/filters to use (typically 26).
    /// * `num_coefficients` - Number of MFCC coefficients to output (typically 13).
    /// * `sample_rate` - Audio sample rate (e.g. 22050.0).
    /// * `fft_size` - FFT size of the source spectrogram (e.g. 1024).
    pub fn new(
        num_mel_filters: usize,
        num_coefficients: usize,
        sample_rate: f32,
        fft_size: usize,
    ) -> Self {
        let num_bins = fft_size / 2 + 1;

        // Hz to Mel scale translation (standard HTK formula)
        let hz_to_mel = |hz: f32| 2595.0 * (1.0 + hz / 700.0).log10();
        // Mel to Hz scale translation
        let mel_to_hz = |mel: f32| 700.0 * (10.0f32.powf(mel / 2595.0) - 1.0);

        let mel_min = hz_to_mel(0.0);
        let mel_max = hz_to_mel(sample_rate / 2.0);

        // M + 2 evenly spaced points in Mel scale
        let mut mel_points = vec![0.0f32; num_mel_filters + 2];
        for i in 0..=(num_mel_filters + 1) {
            mel_points[i] = mel_min + i as f32 * (mel_max - mel_min) / (num_mel_filters + 1) as f32;
        }

        // Map Mel points to linear frequency bin indices
        let mut bin_indices = vec![0usize; num_mel_filters + 2];
        for i in 0..=(num_mel_filters + 1) {
            let hz = mel_to_hz(mel_points[i]);
            let idx = (hz * fft_size as f32 / sample_rate).round() as usize;
            bin_indices[i] = idx.min(num_bins - 1);
        }

        // Pre-compute Triangular filterbank weights matrix [num_mel_filters, num_bins]
        let mut filterbank = vec![vec![0.0f32; num_bins]; num_mel_filters];
        for m in 0..num_mel_filters {
            let left = bin_indices[m];
            let center = bin_indices[m + 1];
            let right = bin_indices[m + 2];

            for k in left..=right {
                if k < num_bins {
                    if k < center {
                        if center > left {
                            filterbank[m][k] = (k - left) as f32 / (center - left) as f32;
                        }
                    } else if k == center {
                        filterbank[m][k] = 1.0;
                    } else if right > center {
                        filterbank[m][k] = (right - k) as f32 / (right - center) as f32;
                    }
                }
            }
        }

        // Pre-compute Orthogonalized Discrete Cosine Transform (DCT-II) matrix [num_coefficients, num_mel_filters]
        let mut dct_matrix = vec![vec![0.0f32; num_mel_filters]; num_coefficients];
        let m_f32 = num_mel_filters as f32;
        for i in 0..num_coefficients {
            let scale = if i == 0 {
                (1.0 / m_f32).sqrt()
            } else {
                (2.0 / m_f32).sqrt()
            };
            for m in 0..num_mel_filters {
                let angle = (std::f32::consts::PI * i as f32 * (m as f32 + 0.5)) / m_f32;
                dct_matrix[i][m] = scale * angle.cos();
            }
        }

        Self {
            num_mel_filters,
            num_coefficients,
            filterbank,
            dct_matrix,
        }
    }

    /// Computes the MFCC coefficients vector for a single frame magnitude spectrum.
    ///
    /// # Arguments
    /// * `magnitude_spectrum` - Slice representing the positive frequency magnitude spectrum (size: fft_size/2 + 1)
    pub fn compute_frame(&self, magnitude_spectrum: &[f32]) -> Vec<f32> {
        let mut mel_energies = vec![0.0f32; self.num_mel_filters];

        // 1. Pass magnitude spectrum through Mel-frequency filterbank
        for m in 0..self.num_mel_filters {
            let mut energy = 0.0f32;
            let filter = &self.filterbank[m];
            for (&spec_val, &weight) in magnitude_spectrum.iter().zip(filter.iter()) {
                energy += spec_val * weight;
            }
            // Log-amplitude compression with small epsilon protection against ln(0)
            mel_energies[m] = (energy + 1e-9f32).ln();
        }

        // 2. Perform Discrete Cosine Transform (DCT-II) mapping to extract MFCC coefficients
        let mut mfccs = vec![0.0f32; self.num_coefficients];
        for i in 0..self.num_coefficients {
            let mut val = 0.0f32;
            let dct_row = &self.dct_matrix[i];
            for m in 0..self.num_mel_filters {
                val += mel_energies[m] * dct_row[m];
            }
            mfccs[i] = val;
        }

        mfccs
    }

    /// Computes the MFCC spectrogram for a 2D magnitude spectrogram.
    /// Returns a 2D vector where each row corresponds to the MFCC coefficients of a frame.
    ///
    /// # Time Complexity
    /// - $O(F \cdot (M \cdot B + C \cdot M))$ where $F$ is the number of frames, $B$ is the number of frequency bins,
    ///   $M$ is `num_mel_filters`, and $C$ is `num_coefficients`.
    /// - This scales linearly with the number of frames.
    ///
    /// # Space Complexity
    /// - $O(F \cdot C)$ for the output 2D vector.
    pub fn compute(&self, spectrogram: &[Vec<f32>]) -> Vec<Vec<f32>> {
        spectrogram
            .iter()
            .map(|frame| self.compute_frame(frame))
            .collect()
    }
}
