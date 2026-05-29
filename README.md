# SonicBridge 核心听觉引擎

[English](README.en.md) | [简体中文](README.md)

面向纯文本大模型（LLM）的极速、超轻量、无预训练模型物理音乐审美与听觉转译中间层工具。

**SonicBridge** 旨在弥合硅基大模型的物理听觉鸿沟（Modal Gap）。它是一个使用纯 Rust 编写的、无任何重型神经网络依赖的数字信号处理（DSP）工具。

通过高性能的音频解码、短时傅里叶时频转换（STFT）、色度向量（Chroma）提取与动态时间规整（DTW）对齐，SonicBridge 能在零点几秒内将一维连续音频波形转译为高信息密度、大模型（甚至是没有多模态能力的纯文本大模型）极其易读的 **LRMD（LLM-Readable Music Descriptor，大模型可读音乐描述符）协议格式报告**。这为陪伴型 AI 智能体（如落雪音乐生态中的 Agent）赋予了“听懂”音乐、细嗅歌手唱腔情感、剖析伴奏空间混响与多版本演绎差异的具身陪伴能力。

---

## 🚀 核心特性 (Key Features)

*   **零模型物理级转译**：完全基于经典 DSP 算法（FFT、HPSS、Chroma），编译后体积仅 **5MB** 左右，无需任何 GPU 显存或数百 MB 的预训练神经网络模型，速度比神经网络转译快数万倍，零部署成本。
*   **三大审美提取方案**：
    *   **方案 A：参数化自适应步长 (Adaptive Steps)**：支持通过命令行参数或 `config.toml` 配置自定义时间步长，高频追踪音响变动。
    *   **方案 B：基于光谱通量的 Onset 事件驱动自适应切分 (Onset-Triggered Partitioning)**：通过计算相邻频谱帧的正向能量差（Spectral Flux），在音频发生瞬态爆发（如鼓点切入、人声爆发、滑音转调）的毫秒级瞬间自适应切分，让审美矩阵完美对齐音乐心跳。
    *   **方案 C：拍子同步乐理重采样 (Beat-Synchronous Resampling)**：基于自相关函数（Autocorrelation）追踪歌曲拍子，以拍（Beat）为边界进行特征合并。
*   **XDG 规范配置管理**：严格遵循 **XDG Base Directory Specification**，支持三级 Fallback 自愈式 TOML 配置加载与动态缓存，杜绝污染用户根目录。
*   **双版本演绎比对 (DTW Aligner)**：物理集成动态时间规整（Dynamic Time Warping）双回溯寻优算法，支持在时间轴非线性扭曲（如歌手自由呼吸、渐慢/渐快）的情况下，完美对齐原唱与翻唱，分析速度与音色细节差异。
*   **Agent 智能同频共听**：无缝对接命令行音乐播放器与客户端。通过推送 LRMD 协议格式报告，让大模型在听歌的指定时间点做出极其专业的乐理剖析与情感反馈。

---

## 📥 安装方式 (Installation)

本项目提供了多种开箱即用的安装方式，您可以选择最适合您系统架构的途径：

### 1. 从 Crates.io 安装 (Rust 生态)
如果您本地配置了 Rust 环境，可直接通过 `cargo` 编译并自动加入系统路径：
```bash
cargo install sonic_bridge
```

### 2. 使用 AUR 安装 (Arch Linux 用户)
如果您使用的是 Arch Linux 及其衍生版（如 Manjaro），可通过您的 AUR 助手直接拉取：
```bash
# 使用 paru
paru -S sonic-bridge-bin

# 使用 yay
yay -S sonic-bridge-bin
```

### 3. 使用 Docker 容器部署
我们也提供了多阶段静态链接打包的极简生产 Docker 镜像：
```bash
docker pull xuepoo/sonic-bridge:latest
```

### 4. 从 GitHub Releases 下载预编译程序
直接前往 [Releases 官方下载页面](https://github.com/Xuepoo/sonic-bridge/releases/latest) 下载适用于您系统的免编译静态二进制包。

---

## 🛠️ 快速上手 (Quick Start)

### 1. 静态编译 (Build from Source)
项目采用纯 Rust 静态链接设计，无需安装任何复杂的外部 C++ 依赖或重型音频包：
```bash
# 1. 克隆代码仓库
git clone https://github.com/Xuepoo/sonic-bridge.git
cd sonic-bridge

# 2. 编译 Release 生产二进制包
cargo build --release

# 3. 运行测试套件验证
cargo test
```
*(编译好的二进制程序 `./target/release/sonic_bridge` 仅约 5MB，可直接运行)*

---

### 2. 命令行调用指南 (CLI Usage)

```bash
# 1. 默认 5.0 秒步长审美分析
./target/release/sonic_bridge "/path/to/song.mp3"

# 2. 启用方案 B：基于 Onset 瞬态事件的自适应审美切分（推荐复杂、高速音乐）
./target/release/sonic_bridge "/path/to/song.mp3" --onset

# 3. 导入自定义 TOML 配置文件
./target/release/sonic_bridge "/path/to/song.mp3" --config "/path/to/my_config.toml"

# 4. 双版本比对（DTW 动态时间规整对齐）
./target/release/sonic_bridge "/path/to/original.mp3" "/path/to/cover_version.mp3"
```

---

## ⚙️ XDG 配置与参数调优 (Configuration)

本工具严格遵循 XDG 规范，优先加载 `$XDG_CONFIG_HOME/sonic-bridge/config.toml`。
详细的配置字段说明、高级环境变量拦截策略以及针对不同音乐流派的参数调优指南，请参阅专门的：
👉 **[SonicBridge 配置与集成手册 (docs/configuration.md)](docs/configuration.md)**。

---

## 📦 容器化与生态发布 (Ecosystem)

*   **Docker 镜像**：预置了多阶段构建的 [Dockerfile](Dockerfile)，可快速生成基于 Alpine Linux 的极简生产镜像：
    ```bash
    docker pull xuepoo/sonic-bridge:latest
    ```
*   **Crates.io**：
    ```toml
    [dependencies]
    sonic-bridge-core = "0.1"
    ```
*   **GitHub Actions CI/CD**：内置了完善的自动化流程，在发布版本 Tag 时（如 `v0.1.0`）会自动构建多架构镜像并分发至 Dockerhub、crates.io 与 AUR。

---

## ⚖️ 开源协议 (License)

本项目基于 [MIT License](LICENSE) 协议开源。欢迎开发者贡献代码、提出 Issue 与 Pull Request。
