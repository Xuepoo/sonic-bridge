# SonicBridge Core Audio Engine

An ultra-fast, lightweight, zero-pretrain-model physical music aesthetic & listening translation middleware designed for pure-text Large Language Models (LLMs).

## 🌐 Overview

**SonicBridge** bridges the physical listening gap (Modal Gap) for AI Agents. It is a digital signal processing (DSP) tool written entirely in Rust, with no heavy machine learning dependencies.

Leveraging high-performance audio decoding, short-time Fourier transform (STFT), Chroma pitch class projection, and Dynamic Time Warping (DTW) alignment, SonicBridge decouples raw 1D waveforms into **LRMD (LLM-Readable Music Descriptor) reports** in a fraction of a second. This empowers AI companions (like Agents in the 落雪音乐 ecosystem) to truly "hear" music, sense vocal timbral emotional shifts, parse arrangement spaces, and compare Cover version performance differences.

---

## 🛠️ Three Adaptive Aesthetic Extraction Strategies

1. **Approach A: Parameterized Adaptive Steps**
   * Customize temporal analysis windows (e.g. 1.0s or 0.5s) via CLI or `config.toml` to capture rapid melodic runs.
2. **Approach B: Spectral Flux Onset-Triggered Partitioning**
   * Compute consecutive spectral frames positive energy flux (Spectral Flux) to dynamically slice acoustic boundaries right at transient attacks (drum entries, glissandos).
3. **Approach C: Beat-Synchronous Resampling**
   * Estimate beat intervals using Autocorrelation functions, merging acoustic descriptors by musical beats or bars.

---

## 🚀 Quick Start

### 1. Build
Statically linked, generating a single **~5MB** binary without any external runtime dependencies:
```bash
cargo build --release
```

### 2. Run Tests
```bash
cargo test
```

### 3. CLI Usage
```bash
# 1. Default 5.0-second interval analysis
./target/release/sonic_bridge "/path/to/song.mp3"

# 2. Enable Approach B: Event-Driven Onset Adaptive Segmentation
./target/release/sonic_bridge "/path/to/song.mp3" --onset

# 3. Load custom TOML config complying with XDG
./target/release/sonic_bridge "/path/to/song.mp3" --config "/path/to/config.toml"

# 4. Cross-Version Comparative Analysis (DTW Warp)
./target/release/sonic_bridge "/path/to/original.mp3" "/path/to/cover.mp3"
```

---

## ⚙️ XDG & TOML Configuration (config.toml)

Strictly complies with the **XDG Base Directory Specification**:
* **Config Directory**: Resolves `$XDG_CONFIG_HOME/sonic-bridge/config.toml` (fallback to `$HOME/.config/sonic-bridge/config.toml`).
* **Cache Directory**: Resolves `$XDG_CACHE_HOME/sonic-bridge/` for storing dynamic spectral matrices.

### Config Example (`config.toml`)
```toml
step_size = 2.0
onset_mode = true
onset_threshold = 0.15
cache_dir = "/path/to/cache"
```

---

## 🦀 Tech Stack Specs

* **Audio Decoding**: `symphonia` (pure-Rust, zero ffpmeg dependency)
* **Spectral Transform**: `rustfft` (SIMD hardware-accelerated FFT)
* **Matrix Calculation**: `ndarray` (multidimensional scientific array)
* **Serialization**: `serde` & `serde_json` & `toml`
