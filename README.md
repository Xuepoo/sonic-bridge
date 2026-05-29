# SonicBridge 核心听觉引擎

面向纯文本大模型（LLM）的极速、超轻量、无预训练模型物理音乐审美与听觉转译中间层工具。

## 🌐 概述

**SonicBridge** 旨在弥合硅基大模型的物理听觉鸿沟（Modal Gap）。它是一个使用纯 Rust 编写的、无任何重型神经网络依赖的数字信号处理（DSP）工具。

通过高性能的音频解码、短时傅里叶时频转换（STFT）、色度向量（Chroma）提取与动态时间规整（DTW）对齐，SonicBridge 能在零点几秒内将一维连续音频波形转译为高信息密度、大模型（甚至是没有多模态能力的纯文本大模型）极其易读的 **LRMD（LLM-Readable Music Descriptor，大模型可读音乐描述符）协议格式报告**。这为陪伴型 AI 智能体（如落雪音乐生态中的 Agent）赋予了“听懂”音乐、细嗅歌手唱腔情感、剖析伴奏空间混响与多版本演绎差异的具身陪伴能力。

---

## 🛠️ 三大核心审美提取方案

1. **方案 A：参数化自适应步长 (Adaptive Steps)**
   * 支持通过命令行参数或 `config.toml` 配置自定义时间步长（如每 1.0s 或 0.5s），高频捕捉极速音响变动。
2. **方案 B：基于光谱通量的 Onset 事件驱动自适应切分 (Onset-Triggered Partitioning)**
   * 纯数学计算相邻频谱帧的正向能量差（Spectral Flux），在音频发生瞬态爆发（如鼓点切入、滑音转调）的毫秒级瞬间自适应切分，使审美矩阵与音乐心跳完全同步。
3. **方案 C：拍子同步乐理重采样 (Beat-Synchronous Resampling)**
   * 基于自相关函数（Autocorrelation）追踪歌曲拍子，以拍（Beat）或小节（Bar）为边界进行特征合并。

---

## 🚀 快速开始

### 1. 编译构建
由于项目采用纯 Rust 静态链接设计，编译体积仅约 5MB，零外部运行时依赖：
```bash
cargo build --release
```

### 2. 运行测试
```bash
cargo test
```

### 3. 命令行调用
```bash
# 1. 默认 5.0 秒步长审美分析
./target/release/sonic_bridge "/path/to/song.mp3"

# 2. 开启方案 B 的 Onset 瞬态事件自适应切分
./target/release/sonic_bridge "/path/to/song.mp3" --onset

# 3. 导入符合 XDG 规范的自定义 TOML 配置文件
./target/release/sonic_bridge "/path/to/song.mp3" --config "/path/to/config.toml"

# 4. 双版本比对（DTW 动态时间规整对齐）
./target/release/sonic_bridge "/path/to/original.mp3" "/path/to/cover.mp3"
```

---

## ⚙️ XDG 规范与配置管理 (config.toml)

本工具严格遵循 **XDG Base Directory Specification** 规范，杜绝污染用户根目录：
* **配置文件路由**：优先寻检加载 `$XDG_CONFIG_HOME/sonic-bridge/config.toml`（fallback 至 `$HOME/.config/sonic-bridge/config.toml`）。
* **缓存数据路由**：自动重定向至 `$XDG_CACHE_HOME/sonic-bridge/`，用于存储分析过程的临时高维特征矩阵。

### 配置文件示例 (`config.toml`)
```toml
step_size = 2.0
onset_mode = true
onset_threshold = 0.15
cache_dir = "/path/to/cache"
```

---

## 🦀 技术栈选型

* **音频解码**：`symphonia` (纯 Rust 编写的多格式极速音频解包解码库)
* **时频转换**：`rustfft` (支持 SIMD 硬件加速的极速傅里叶变换库)
* **矩阵计算**：`ndarray` (高性能多维科学计算矩阵库)
* **序列化**：`serde` & `serde_json` & `toml`
