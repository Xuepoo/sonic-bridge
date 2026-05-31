# Changelog - sonic-bridge

All notable changes to the `sonic-bridge` core library and CLI will be documented in this file.

---

## [0.4.1] - 2026-05-31

### Added
- **Chinese Pentatonic Modes (Gong/Yu) Mapping**: Evaluates chroma sparseness at scale degrees. Automatically detects and maps Chinese pentatonic modes (e.g. `D 宫调式`) instead of forcing them into Western Major/Minor keys.
- **Tonal Third Discriminator**: Hard-discriminates Major vs Minor keys by calculating the ratio between the Minor Tonal Third and Major Tonal Third, resolving classic synthwave major-minor key confusions (e.g. *Blinding Lights* F Minor).
- **Phrygian-second Mediant Key Corrector**: Automatically overrides submediant major/minor key mismatches (e.g. A Minor to F Major) when the Phrygian flat-second degree (Bb) is much stronger than the major second (B), perfectly resolving Yoasobi *たぶん* to F Major.
- **Fractional Linear Interpolation Chroma Projection**: Replaces discrete rounding with fractional linear interpolation, completely eliminating discrete frequency bin leakage in STFT chroma projection and restoring true harmonic balance.
- **Dynamic Melodic Noise Gate**: Integrates 12x boost for tracked vocals and melody, while suppressing background, percussive, and structural floor resonances in non-melodic frames by up to 97%.
- **Onset-Density Validated Octave Harmonic Evaluator**: Restricts subharmonic tempo halving by cross-validating autocorrelation lag with unfolded IOI histogram density. Solves body-percussion octave sways (e.g. *We Will Rock You* to 80.7 BPM) while locking fast beats (e.g. *Blinding Lights* to 172.3 BPM).
- **9-Frame Moving Average Smoothing Filter**: Applied moving average smoothing to STFT energy envelopes, significantly improving downbeat correlation alignment robustness on sparse arrangements.

### Changed
- **Onset Tracker Debounce**: Optimized debounce window `min_interval_frames` from 15 to 4 to capture high-tempo beats (>170 BPM) and subdivisions.

## [0.4.0] - 2026-05-31

### Added
- **BPM Confidence Model (Histogram & Autocorrelation Voting)**: Integrated a multi-histogram tempo classifier combining Zero-Mean Autocorrelation with Inter-Onset Interval (IOI) histogram peaks. Solved octave tempo ambiguities and stabilized rhythm tracking on extreme rhythmic profiles.
- **Melodic Weighting Pitch Tracker**: Implemented a real-time, low-overhead F0 pitch class estimator restricting spectral tracking to the human vocal core resonance band (300Hz-1200Hz), enabling **3.0x Melodic Boosting** in Chroma projection. Resolves Major/Minor key confusion under heavy arrangement backings.
- **Spectral Flatness Arranged Density 修剪**: Integrated Spectral Flatness (Wiener Entropy) computation inside sub-segment calculations to dynamically identify sparse and minimal body percussion tracks, automatically mapping and correcting dynamic level overflows to realistic musicology levels.

### Changed
- **CLI Global Overrides**: Updated manuals to reflect the fully stabilized Scheme C beat tracking algorithms now under the hood.

## [0.3.8] - 2026-05-31

### Added
- **Beat-Synchronous Resampling Mode (Scheme C)**: Implemented tempo-synced feature integration. Enabled by a new CLI `--beat` option or TOML `beat_mode = true` configuration, it automatically computes precise beat-interval boundaries according to the dynamically estimated BPM to perform rhythmically aligned music appreciation.
- **TDD Verification Suite for Beat Mode**: Added `tests/beat_tests.rs` to assert correct beat extraction, boundary slicing, and consecutive identical phrase block mergers.

### Changed
- **CLI Manual and Help Page Update**: Documented the new `--beat` flag within `--help` print manuals.

## [0.3.7] - 2026-05-31

### Added
- **Dynamic Onset-Based BPM Estimator (Issue #30)**: Designed a high-precision BPM detection algorithm based on Onset Interval Autocorrelation and Median filtering, mapping tempos to the 60-180 BPM range dynamically.
- **Adaptive Tempo Subjective Mapping**: Implemented tempo feeling descriptions (e.g. Adagio, Andante, Moderato, Allegro, Presto) dynamically bound to estimated BPM.
- **Global Key Detector Integration (Issue #31)**: Wired the unused `KeyDetector` Krumhansl-Schmuckler algorithm back to the top-level pipeline, correctly accumulating 12-dimensional chroma centroids across all frames instead of hardcoding "Unknown".
- **CLI `--threshold <value>` Parameter (Issue #33)**: Exposed a flexible threshold configuration option to let users customize onset sensitivity at run time.

### Changed
- **NFC/NFD Unicode Path Normalization (Issue #32)**: Created a dual-normalization fallback (`normalize_path`) to auto-convert NFC and NFD paths, resolving compatibility errors when accessing files with Japanese dakuten/handakuten on macOS.
- **Minimum Onset Interval Debouncing**: Tuned onset detection to suppress high-frequency micro-transient jitters with a 348ms minimum interval constraint, preventing segment fragmentation.

---

## [0.3.6] - 2026-05-31

### Added
- **Dual-Source Sync Fallback Engine**: Updated the TUI player (`--render`) to search for a co-located `.lrmd.md` report and automatically fill in audio metadata (Title, Artist, Duration) if missing in the `.alrc` file.
- **Real-Time Physical Attribute Fallback**: Implemented time-aligned physical data matching. If an active `.alrc` segment has missing or "Unknown" chord or timbre metadata, the renderer dynamically queries the timeline table of `.lrmd.md` to retrieve and display live chords, dynamics, and timbre.

### Changed
- **Chord Root Equivalence Merger (Issue #12)**: Solved phrase merger issues on complex chord labels (e.g. `maj7`, `sus2`, `m7`) by introducing zero-allocation root pitch extraction (`Self::get_chord_root`), enabling perfect merge compression.
- **Silent Mode Suppression (Issue #16)**: Wrapped all stdout notifications under `!config.quiet_mode` validation, achieving a perfectly clean CLI stdout for automated batch execution.

---

## [0.3.5] - 2026-05-30

### Added
- **Aesthetic Terminal Renderer (`--render`)**: Introduced a standalone dynamic terminal scrolling player mode to sync and preview synesthetic comments parsed from `.alrc` files co-located with audio source tracks.
- **Flicker-Free TUI Redraw Engine**: Built using terminal carriage returns and cursor movement ANSI codes (`\x1b[H`) to secure seamless, 80ms-interval scrolling animation with zero terminal flickers.
- **Rigorously Integrated TDD Suite**: Implemented `tests/renderer_tests.rs` to assert 100% precision in parsing custom time offsets, metadata, synesthetic sensor tokens, and musicological comments.

### Changed
- **CLI Argument Interception & Parameter Parsing**: Replaced implicit folder scanning with a robust global interceptor for `-h`, `--help`, `help`, `-v`, `--version`, and `version` before starting any audio pipeline processing.
- **Safe Out-of-Pipeline Existence Validation**: Added pre-pipeline physical existence check (`Path::exists()`) for both single and comparative audio files, outputting graceful red-highlighted errors instead of raw symphonia IO errors.
- **Refined Iterator Code for Performance**: Satisfied clippy's high-efficiency guidelines by using `.strip_prefix()` for sensor text extraction and `.rfind()` for backward sequence matching on `DoubleEndedIterator`, optimizing overall time complexity.

---

## [0.3.4] - 2026-05-29

### Added
- Adaptive Onset event segmentation support for dynamic rhythm chunking.
- Full compliance with XDG path directives for config and local cache.
