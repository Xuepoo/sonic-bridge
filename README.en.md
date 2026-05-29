# SonicBridge Core Audio Engine

[English](README.en.md) | [简体中文](README.md)

An ultra-fast, lightweight, zero-pretrain-model physical music aesthetic & listening translation middleware designed for pure-text Large Language Models (LLMs).

**SonicBridge** bridges the physical listening gap (Modal Gap) for AI Agents. It is a digital signal processing (DSP) tool written entirely in Rust, with no heavy machine learning dependencies.

Leveraging high-performance audio decoding, short-time Fourier transform (STFT), Chroma pitch class projection, and Dynamic Time Warping (DTW) alignment, SonicBridge decouples raw 1D waveforms into **LRMD (LLM-Readable Music Descriptor) reports** in a fraction of a second. This empowers AI companion agents to truly "hear" music, sense vocal timbral emotional shifts, parse arrangement spaces, and compare Cover version performance differences.

---

## 🚀 Key Features

*   **Zero-Model Translation**: Built entirely on classic DSP algorithms (FFT, HPSS, Chroma), yielding a statically compiled binary of only **~5MB** with zero heavy GPU or neural network runtime dependencies.
*   **Three Aesthetic Slicing Strategies**:
    *   **Approach A: Parameterized Adaptive Steps**: Customize temporal analysis windows via CLI or `config.toml` to track melodic changes.
    *   **Approach B: Spectral Flux Onset-Triggered Partitioning**: Compute consecutive spectral frames positive energy flux to dynamically slice acoustic boundaries right at transient attacks (drum entries, glissandos).
    *   **Approach C: Beat-Synchronous Resampling**: Estimate beat intervals using Autocorrelation functions, merging acoustic descriptors by musical beats.
*   **XDG Specification Compliance**: Fully complies with **XDG Base Directory Specifications** supporting self-healing TOML configuration directories.
*   **Cross-Version DTW Alignment**: Employs Dynamic Time Warping dual-backtracking pathfinding, enabling robust temporal alignment of Covers and Originals under non-linear tempo changes.
*   **AI Agent Co-Listening Co-Intelligence**: Designed to seamlessly interface with CLI music players and clients. By generating standard `.lrmd.md` reports, text-only LLMs get full synesthetic ability to act as real-time musicological companions.

---

## 📥 Installation

SonicBridge provides multiple out-of-the-box installation strategies. Select the method that best matches your system layout:

### 1. Build via Cargo (Rust Ecosystem)
If you have a Rust toolchain configured locally, compile and install it globally via `cargo`:
```bash
cargo install sonic-bridge-core
```

### 2. Install from AUR (Arch Linux & Manjaro Users)
Statically compiled packages are natively distributed inside the Arch User Repository:
```bash
# Using paru
paru -S sonic-bridge-bin

# Using yay
yay -S sonic-bridge-bin
```

### 3. Deploy via Docker
A minimal runtime image built using multi-stage Alpine static linking compilation is published:
```bash
docker pull xuepoo/sonic-bridge:latest
```

### 4. Fetch Pre-compiled Binaries from GitHub Releases
Navigate to the [Official GitHub Releases Page](https://github.com/Xuepoo/sonic-bridge/releases/latest) to directly fetch statically linked standalone binaries compiled for your host architecture.

---

## 🛠️ Quick Start

### 1. Build from Source
Statically linked, generating a single, self-contained binary without any external runtime dependencies:
```bash
# 1. Clone Repository
git clone https://github.com/Xuepoo/sonic-bridge.git
cd sonic-bridge

# 2. Compile Release Binary
cargo build --release

# 3. Run Tests
cargo test
```
*(The compiled executable `./target/release/sonic-bridge` is roughly 5MB)*

---

### 2. CLI Usage Guide

```bash
# 1. Default 5.0-second interval analysis
sonic-bridge "/path/to/song.mp3"

# 2. Enable Approach B: Event-Driven Onset Adaptive Segmentation
sonic-bridge "/path/to/song.mp3" --onset

# 3. Load custom TOML config
sonic-bridge "/path/to/song.mp3" --config "/path/to/my_config.toml"

# 4. Cross-Version Comparative Analysis (DTW Warp)
sonic-bridge "/path/to/original.mp3" "/path/to/cover_version.mp3"
```

---

## ⚙️ XDG & Parameter Tuning (Configuration)

Prioritizes loading `$XDG_CONFIG_HOME/sonic-bridge/config.toml`.
For detailed field explanations, XDG environment interceptions, and musical style tuning metrics, please refer to:
👉 **[SonicConfig User & Configuration Guide (docs/configuration.md)](docs/configuration.md)**.

---

## 📦 Containerization & Deployment

*   **Docker Registry**: Uses a multi-stage [Dockerfile](Dockerfile) to compile a minimal runtime container based on Alpine Linux:
    ```bash
    docker pull xuepoo/sonic-bridge:latest
    ```
*   **Crates.io**:
    ```toml
    [dependencies]
    sonic-bridge-core = "0.1"
    ```
*   **GitHub Actions CI/CD**: Automatically builds multi-platform binaries, publishing to Dockerhub, crates.io, and AUR upon version tags (e.g. `v0.1.0`).

---

## ⚖️ License

Stably licensed under the [MIT License](LICENSE). Contributions, issues, and Pull Requests are welcome!
