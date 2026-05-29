use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::sync::Arc;

/// StftEngine is a lightweight, high-performance Short-Time Fourier Transform (STFT) engine.
/// It pre-allocates FFT plans and Hanning window vectors to optimize execution speed.
pub struct StftEngine {
    window_size: usize,
    hop_size: usize,
    fft: Arc<dyn Fft<f32>>,
    hanning_window: Vec<f32>,
}

impl StftEngine {
    /// Creates a new `StftEngine` instance.
    ///
    /// # Arguments
    /// * `window_size` - The size of the FFT window (e.g., 1024).
    /// * `hop_size` - The step size between consecutive frames (e.g., 512).
    pub fn new(window_size: usize, hop_size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(window_size);

        // Pre-compute Hanning window to avoid per-compute allocation/math
        let hanning_window: Vec<f32> = (0..window_size)
            .map(|i| {
                0.5 * (1.0
                    - (2.0 * std::f32::consts::PI * i as f32 / (window_size - 1) as f32).cos())
            })
            .collect();

        Self {
            window_size,
            hop_size,
            fft,
            hanning_window,
        }
    }

    /// Computes the Short-Time Fourier Transform of a 1D signal.
    /// Returns a 2D magnitude spectrogram: Vec of frames, each containing (window_size / 2 + 1) bins.
    ///
    /// # Time Complexity
    /// - $O(\frac{N}{H} \cdot W \log W)$ where $N$ is the signal length, $W$ is `window_size`, and $H$ is `hop_size`.
    /// - With fixed $W, H$, the time complexity scales linearly with signal length: $O(N)$.
    ///
    /// # Space Complexity
    /// - $O(\frac{N}{H} \cdot W)$ for the output spectrogram.
    /// - Auxiliary workspace space complexity is $O(W)$ for the buffer.
    pub fn compute(&self, signal: &[f32]) -> Result<Vec<Vec<f32>>, String> {
        if self.window_size == 0 {
            return Err("Window size cannot be zero".to_string());
        }
        if self.hop_size == 0 {
            return Err("Hop size cannot be zero".to_string());
        }
        if signal.len() < self.window_size {
            return Ok(Vec::new());
        }

        let num_bins = self.window_size / 2 + 1;
        let mut spectrogram = Vec::new();

        // Reusable scratch buffer for FFT processing to avoid excessive allocations
        let mut buffer = vec![Complex::new(0.0f32, 0.0f32); self.window_size];

        let mut start = 0;
        while start + self.window_size <= signal.len() {
            // Apply Hanning window
            for i in 0..self.window_size {
                buffer[i] = Complex::new(signal[start + i] * self.hanning_window[i], 0.0);
            }

            // Forward FFT (in-place)
            self.fft.process(&mut buffer);

            // Extract magnitude of positive frequencies (first half + Nyquist bin)
            let magnitudes: Vec<f32> = buffer[0..num_bins].iter().map(|c| c.norm()).collect();

            spectrogram.push(magnitudes);
            start += self.hop_size;
        }

        Ok(spectrogram)
    }
}
