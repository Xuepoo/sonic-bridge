use ndarray::Array2;

/// A struct for performing Dynamic Time Warping (DTW) alignment on 1D sequences.
pub struct DtwAligner;

impl DtwAligner {
    /// Creates a new instance of `DtwAligner`.
    pub fn new() -> Self {
        Self
    }

    /// Aligns two 1D sequences using Dynamic Time Warping.
    ///
    /// Computes the local and cumulative distance matrices and backtracks
    /// from `(n-1, m-1)` to `(0, 0)` to find the optimal warping path.
    ///
    /// Time Complexity: O(N * M)
    /// Space Complexity: O(N * M)
    ///
    /// # Arguments
    /// * `seq_a` - The first sequence of energy/chroma values.
    /// * `seq_b` - The second sequence of energy/chroma values.
    ///
    /// # Returns
    /// A vector of tuples `Vec<(usize, usize)>` representing the optimal warping path.
    pub fn align(&self, seq_a: &[f32], seq_b: &[f32]) -> Vec<(usize, usize)> {
        let n = seq_a.len();
        let m = seq_b.len();

        if n == 0 || m == 0 {
            return Vec::new();
        }

        // Cumulative distance matrix
        let mut cost = Array2::<f32>::zeros((n, m));

        // Base case: (0, 0)
        cost[[0, 0]] = (seq_a[0] - seq_b[0]).abs();

        // Populate the first column
        for i in 1..n {
            cost[[i, 0]] = cost[[i - 1, 0]] + (seq_a[i] - seq_b[0]).abs();
        }

        // Populate the first row
        for j in 1..m {
            cost[[0, j]] = cost[[0, j - 1]] + (seq_a[0] - seq_b[j]).abs();
        }

        // Compute cumulative costs for the rest of the matrix
        for i in 1..n {
            for j in 1..m {
                let diff = (seq_a[i] - seq_b[j]).abs();
                let min_prev = cost[[i - 1, j]]
                    .min(cost[[i, j - 1]])
                    .min(cost[[i - 1, j - 1]]);
                cost[[i, j]] = diff + min_prev;
            }
        }

        // Backtrack path from (n-1, m-1) to (0, 0)
        let mut path = Vec::new();
        let mut i = n - 1;
        let mut j = m - 1;
        path.push((i, j));

        while i > 0 || j > 0 {
            if i == 0 {
                j -= 1;
            } else if j == 0 {
                i -= 1;
            } else {
                let prev_diag = cost[[i - 1, j - 1]];
                let prev_left = cost[[i, j - 1]];
                let prev_up = cost[[i - 1, j]];

                // Prioritize diagonal transition to enforce causal temporal progression
                if prev_diag <= prev_left && prev_diag <= prev_up {
                    i -= 1;
                    j -= 1;
                } else if prev_left <= prev_up {
                    j -= 1;
                } else {
                    i -= 1;
                }
            }
            path.push((i, j));
        }

        path.reverse();
        path
    }
}

impl Default for DtwAligner {
    fn default() -> Self {
        Self::new()
    }
}
