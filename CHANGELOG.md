# Changelog - sonic-bridge

All notable changes to the `sonic-bridge` core library and CLI will be documented in this file.

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
