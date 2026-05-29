# SonicConfig: SonicBridge Configuration & Integration Guide

This document describes the XDG directory structure, CLI configs, custom TOML specifications, and tuning metrics for `sonic-bridge`.

---

## 📂 1. XDG Base Directory Compliance

`sonic-bridge` strictly adheres to the **XDG Base Directory Specification** to ensure clean user environments on Linux and Unix systems. It uses the following resolution sequence to load config files and manage runtime cache matrices:

### Config Directory Resolution Sequence
1. **Override Flag**: If `--config <path>` is supplied in the CLI, `sonic-bridge` loads the specified path immediately.
2. **XDG Environment Variable**: Looks up `$XDG_CONFIG_HOME/sonic-bridge/config.toml`.
3. **Primary Standard Fallback**: Looks up `$HOME/.config/sonic-bridge/config.toml`.
4. **Hardcoded Fallback**: If no file is found, it automatically instantiates standard memory defaults (自愈机制) to prevent program failure.

### Cache Directory Resolution Sequence
1. **Toml Definition**: Loads the `cache_dir` specified inside `config.toml`.
2. **XDG Environment Variable**: Looks up `$XDG_CACHE_HOME/sonic-bridge/`.
3. **Primary Standard Fallback**: Looks up `$HOME/.cache/sonic-bridge/`.

---

## ⚙️ 2. TOML Configuration File (`config.toml`)

Here is a standard production configuration file with all parameters explained:

```toml
# =====================================================================
# SonicBridge User Configuration Reference (config.toml)
# =====================================================================

# ---------------------------------------------------------------------
# 1. Parameterized Step Size (Approach A)
# ---------------------------------------------------------------------
# Used when `onset_mode` is set to false. Defines the temporal window
# step size (in seconds) to slice the music analysis segmentations.
# e.g., 2.0 means generating an aesthetic description block every 2 seconds.
# Range: 0.1 - 60.0 (Default: 5.0)
step_size = 2.0

# ---------------------------------------------------------------------
# 2. Onset Event-Driven Slicing (Approach B)
# ---------------------------------------------------------------------
# When enabled, standard step-slicing is disabled. Slicing boundaries
# are dynamically triggered by positive changes in consecutive frames
# Spectral Flux energy (Onset detection). Highly recommended for tempo-fluid,
# complex, and rapid-beat tracks to achieve perfect beat alignment.
# Options: true, false (Default: false)
onset_mode = true

# ---------------------------------------------------------------------
# 3. Onset Attack Threshold
# ---------------------------------------------------------------------
# Controls the sensitivity of the Onset peak transient detector.
# Lower threshold makes it highly sensitive (triggering on mild transients).
# Higher threshold triggers only on prominent drum strikes or major drops.
# Range: 0.05 - 1.0 (Default: 0.5)
onset_threshold = 0.15

# ---------------------------------------------------------------------
# 4. Global Cache Override
# ---------------------------------------------------------------------
# Custom path to override XDG_CACHE_HOME for storing transient STFT matrices.
# cache_dir = "/tmp/sonic-bridge-cache"
```

---

## 🎛️ 3. Musical Style Tuning Guidelines

Depending on the track genre, fine-tuning the `config.toml` config dramatically enhances the output **LRMD (LLM-Readable Music Descriptor)** accuracy for companion Agents.

### Case A: Electronic, Math Rock & Hyper-Tempo Tracks (e.g. *いよわ - 1000年生きてる*)
* **Goal**: Capture micro-second beat transients and hyper-active harmonic shifts.
* **Suggested Config**:
  ```toml
  onset_mode = true
  onset_threshold = 0.12 # Ultra-sensitive to capture drum attacks
  ```

### Case B: Ambient, Classical Piano & Cinematic Soundtracks (e.g. *Joe Hisaishi*)
* **Goal**: Capture broad, flowing harmonic movements without hyperactive fragmentation.
  * **Suggested Config**:
  ```toml
  onset_mode = false
  step_size = 4.0 # Flowing segment updates every 4 seconds
  ```

### Case C: Vocal Accapella & Dry Acoustic Pop
* **Goal**: Focus on vocal presence and chord resolutions.
  * **Suggested Config**:
  ```toml
  onset_mode = true
  onset_threshold = 0.35 # Mild sensitivity to target main vocal entries
  ```

---

## 🤖 4. AI Agent System Integration

AI companion agents (e.g., `Lumina` or `Hermes`) hook into the `sonic-bridge` execution pipeline by reading the generated `.lrmd.md` file.

```markdown
# Flowing Pipeline Integration
[ alx play ]
    │
    ├──► [ trigger sonic-bridge --onset ]
    │          │
    │          └─► Generates <track>.lrmd.md (0.05s execution)
    │
    └──► [ Agent loads .lrmd.md into context ]
               │
               └─► Companion dialog generation during playback
```

Agents use the JSON structure or formatted Markdown table inside `.lrmd.md` as context variables.
For advanced integration details, please refer to the `sonic-bridge-skills` repository.
