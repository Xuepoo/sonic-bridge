# Changelog - sonic-bridge

All notable changes to the `sonic-bridge` core library and CLI will be documented in this file.

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
