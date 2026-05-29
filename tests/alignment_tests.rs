use sonic_bridge_core::alignment::dtw::DtwAligner;

#[test]
fn test_dtw_alignment() {
    // Two simple 1D sequences representing slightly warped energy lines
    // seq_a: length 6
    let seq_a = vec![1.0, 2.0, 3.0, 4.0, 3.0, 2.0];
    // seq_b: length 7 (warped by inserting 1.5 and 3.5 to simulate time stretching)
    let seq_b = vec![1.0, 1.5, 2.0, 3.0, 4.0, 3.5, 2.0];

    let aligner = DtwAligner::new();
    let path = aligner.align(&seq_a, &seq_b);

    // Assert boundary conditions
    assert_eq!(
        path.first().unwrap(),
        &(0, 0),
        "The path must start at (0, 0)"
    );
    assert_eq!(
        path.last().unwrap(),
        &(5, 6),
        "The path must end at (n-1, m-1)"
    );

    // Verify alignment causality and monotonicity:
    // 1. Monotonicity: path[k] = (i, j) -> i and j must be non-decreasing.
    // 2. Continuity: path[k+1] - path[k] must be in {(0, 1), (1, 0), (1, 1)}.
    for w in path.windows(2) {
        let (i1, j1) = w[0];
        let (i2, j2) = w[1];

        let diff_i = i2 as isize - i1 as isize;
        let diff_j = j2 as isize - j1 as isize;

        assert!(
            diff_i >= 0 && diff_j >= 0,
            "Path must be non-decreasing: ({}, {}) -> ({}, {})",
            i1,
            j1,
            i2,
            j2
        );
        assert!(
            (diff_i == 0 && diff_j == 1)
                || (diff_i == 1 && diff_j == 0)
                || (diff_i == 1 && diff_j == 1),
            "Path step must be within step size 1: ({}, {}) -> ({}, {})",
            i1,
            j1,
            i2,
            j2
        );
    }

    // Verify causal matching: the peaks of the two energy envelopes (value 4.0) should align.
    // seq_a[3] = 4.0 and seq_b[4] = 4.0.
    assert!(
        path.contains(&(3, 4)),
        "Peak alignment failed: the path {:?} should align peak indices (3, 4)",
        path
    );
}
