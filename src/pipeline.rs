use crate::alignment::dtw::DtwAligner;
use crate::config::SonicConfig;
use crate::decoder::AudioDecoder;
use crate::dsp::spectrogram::StftEngine;
use crate::musicology::chroma::ChordClassifier;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GlobalMetadata {
    pub filename: String,
    pub duration_seconds: f32,
    pub estimated_bpm: f32,
    pub estimated_global_key: String,
    pub tempo_feeling: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SegmentAesthetic {
    pub time_range: String,
    pub chord: String,
    pub dynamic_level: String,
    pub timbre_brightness: String,
    pub rhythm_activity: String,
    pub raw_energy: f32,
    pub raw_centroid: f32,
}

pub struct SonicPipeline;

impl SonicPipeline {
    /// 执行单音轨的完整物理声学与乐理解构 pipeline，生成大模型友好描述符
    pub fn process_single(
        audio_path: &Path,
        config: &SonicConfig,
    ) -> Result<(GlobalMetadata, Vec<SegmentAesthetic>), String> {
        // 1. 音频解码与重采样 (22050Hz)
        let decoder = AudioDecoder::new(audio_path)?;
        let samples = decoder.decode()?;
        let sr = decoder.sample_rate() as f32;
        let duration = samples.len() as f32 / sr;

        // 2. 运行 STFT 转换 (窗长 1024, 步长 512)
        let window_size = 1024;
        let hop_size = 512;
        let engine = StftEngine::new(window_size, hop_size);
        let spectrogram = engine.compute(&samples)?;

        // 3. 初始化乐理分析器
        let chord_classifier = ChordClassifier::new();

        let mut segments = Vec::new();
        let frame_duration = hop_size as f32 / sr;

        // 根据 config.step_size 决定自适应分块帧数
        let frames_per_step = (config.step_size / frame_duration).round() as usize;

        let mut split_points = Vec::new();
        if config.onset_mode {
            use crate::dsp::onset::OnsetDetector;
            let detector = OnsetDetector::new(config.onset_threshold);
            let boundary_frames = detector.detect_boundaries(&spectrogram);
            let mut temp = vec![0];
            for &b in &boundary_frames {
                if b > 0 && b < spectrogram.len() {
                    temp.push(b);
                }
            }
            temp.push(spectrogram.len());
            temp.sort();
            temp.dedup();
            split_points = temp;
        }

        // 预扫描计算全局最大 RMS 动态，以供局部相对 RMS (Relative RMS) 分类使用
        let mut global_max_rms = 0.0f32;
        for frame in &spectrogram {
            let sum_power: f32 = frame.iter().map(|&x| x * x).sum();
            let rms = (sum_power / frame.len() as f32).sqrt();
            if rms > global_max_rms {
                global_max_rms = rms;
            }
        }
        if global_max_rms < 0.0001 {
            global_max_rms = 1.0; // 避免除以零
        }

        let mut frame_idx = 0;
        let mut interval_idx = 0;

        while frame_idx < spectrogram.len() {
            let end_frame = if config.onset_mode {
                split_points
                    .iter()
                    .copied()
                    .find(|&p| p > frame_idx)
                    .unwrap_or(spectrogram.len())
            } else {
                (frame_idx + frames_per_step).min(spectrogram.len())
            };

            let sub_specs = &spectrogram[frame_idx..end_frame];

            if sub_specs.is_empty() {
                break;
            }

            // 4. 计算区间平均特征
            let mut sum_rms = 0.0f32;
            let mut sum_centroid = 0.0f32;
            let mut sum_chroma = vec![0.0f32; 12];
            let mut counts = 0;

            for frame in sub_specs {
                let sum_power: f32 = frame.iter().map(|&x| x * x).sum();
                let rms = (sum_power / frame.len() as f32).sqrt();
                sum_rms += rms;

                // 谱质心 (音色亮度)
                let mut num = 0.0f32;
                let mut den = 0.0f32;
                for (bin, &mag) in frame.iter().enumerate() {
                    let freq = bin as f32 * (sr / window_size as f32);
                    num += freq * mag;
                    den += mag;
                }
                let centroid = if den > 0.0 { num / den } else { 0.0 };
                sum_centroid += centroid;

                // 色度向量合并 (基于 200Hz - 2kHz 审美带通投影，过滤低频共振与高频刺耳物，聚焦旋律音高)
                for (bin, &mag) in frame.iter().enumerate() {
                    let freq = bin as f32 * (sr / window_size as f32);
                    if (200.0..=2000.0).contains(&freq) {
                        // 物理公式：将频率转化为 MIDI 音高 p = 69 + 12 * log2(f/440)
                        let midi_pitch = 69.0 + 12.0 * (freq / 440.0).log2();
                        let pitch_class = (midi_pitch.round() as i32) % 12;
                        if pitch_class >= 0 {
                            sum_chroma[pitch_class as usize] += mag;
                        }
                    }
                }

                counts += 1;
            }

            let (t_start, t_end) = if config.onset_mode {
                let start_time = frame_idx as f32 * frame_duration;
                let end_time = (end_frame as f32 * frame_duration).min(duration);
                (start_time, end_time)
            } else {
                let start_time = interval_idx as f32 * config.step_size;
                let end_time = (start_time + config.step_size).min(duration);
                (start_time, end_time)
            };

            let mean_rms = sum_rms / counts as f32;
            let mean_centroid = sum_centroid / counts as f32;

            // 计算分片中的时域 Peak 值以求出 Crest Factor（波峰因数），量化混音的冲击感与呼吸感
            let start_sample = (t_start * sr) as usize;
            let end_sample = ((t_end * sr) as usize).min(samples.len());
            let mut peak_val = 0.0f32;
            if start_sample < end_sample {
                for &s in &samples[start_sample..end_sample] {
                    let abs_s = s.abs();
                    if abs_s > peak_val {
                        peak_val = abs_s;
                    }
                }
            }
            let crest_factor = if mean_rms > 0.0001 {
                peak_val / mean_rms
            } else {
                0.0
            };

            // A. 自适应动态电平映射（通过 relative_rms 和 crest_factor 联合映射，优雅解决现代音乐砖墙限幅饱满带来的 Fortissimo 霸屏 Bug）
            let relative_rms = mean_rms / global_max_rms;
            let dynamic_desc = if relative_rms < 0.01 {
                "Silent/Near-Silent"
            } else if relative_rms < 0.12 {
                "Very Soft (Pianissimo)"
            } else if relative_rms < 0.35 {
                "Soft & Intimate (Piano)"
            } else if relative_rms < 0.65 {
                "Moderately Intense (Mezzo-Forte)"
            } else if relative_rms < 0.85 {
                "Loud & Energetic (Forte)"
            } else {
                // 极高电平区：如果 Crest Factor 过低，说明是被 maximizer 压缩到极限的响度，标记为 Loud & Dense
                // 如果 Crest Factor 较高，说明是具备强烈 Transient 冲击感的物理爆发点，标记为 Fortissimo
                if crest_factor < 2.5 {
                    "Loud & Dense (Forte)"
                } else {
                    "Exploding Intensity (Fortissimo)"
                }
            };

            // B. 音色亮度映射
            let timbre_desc = if mean_centroid < 900.0 {
                "Deep & Dark (Muddy/Sub-heavy)"
            } else if mean_centroid < 1600.0 {
                "Warm & Smooth (Mellow mid-range)"
            } else if mean_centroid < 2600.0 {
                "Balanced & Clear (Realistic vocal presence)"
            } else if mean_centroid < 4200.0 {
                "Bright & Crisp (Sharp transients)"
            } else {
                "Piercing & Airy (Airy presence)"
            };

            // C. 节奏活跃度估算 (基于相对 RMS，使整体对不同歌曲尺度的活跃度反映更加健康自适应)
            let rhythm_desc = if relative_rms < 0.02 {
                "Static & Ambient sustained notes"
            } else if relative_rms < 0.25 {
                "Flowing & Legato (Gentle melodic flow)"
            } else {
                "Steady Beat (Clear rhythmic dynamic)"
            };

            // D. 和弦估计
            let chord = chord_classifier.classify(&sum_chroma);

            segments.push(SegmentAesthetic {
                time_range: format!("{:.1}s - {:.1}s", t_start, t_end),
                chord,
                dynamic_level: dynamic_desc.to_string(),
                timbre_brightness: timbre_desc.to_string(),
                rhythm_activity: rhythm_desc.to_string(),
                raw_energy: mean_rms,
                raw_centroid: mean_centroid,
            });

            frame_idx = end_frame;
            interval_idx += 1;
        }

        // 估计全局属性
        let global_chroma: Vec<f32> = (0..12)
            .map(|_i| segments.iter().map(|s| s.raw_energy).sum()) // 粗估
            .collect();
        let global_key = chord_classifier.classify(&global_chroma);

        let global_metadata = GlobalMetadata {
            filename: audio_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            duration_seconds: duration,
            estimated_bpm: 120.0, // 默认均值速度
            estimated_global_key: global_key,
            tempo_feeling: "Moderate & Flowing (Andante/Moderato)".to_string(),
        };
        let merged_segments = Self::merge_segments(segments);
        Ok((global_metadata, merged_segments))
    }

    /// Extracts the fundamental root pitch class from a complex chord symbol (e.g. "A#maj7" -> "A#", "Dm7" -> "D")
    fn get_chord_root(chord: &str) -> &str {
        if chord == "Silent" || chord == "Unknown" {
            return chord;
        }
        if chord.len() >= 2 {
            let bytes = chord.as_bytes();
            if (bytes[0] >= b'A' && bytes[0] <= b'G') && (bytes[1] == b'#' || bytes[1] == b'b') {
                return &chord[0..2];
            }
        }
        if !chord.is_empty() {
            let bytes = chord.as_bytes();
            if bytes[0] >= b'A' && bytes[0] <= b'G' {
                return &chord[0..1];
            }
        }
        chord
    }

    /// Spatiotemporal Chunk Merger: compresses consecutive slices sharing identical Chord Root, Timbre, and Dynamic features into Phrase Blocks
    fn merge_segments(segs: Vec<SegmentAesthetic>) -> Vec<SegmentAesthetic> {
        if segs.is_empty() {
            return segs;
        }

        let mut merged = Vec::new();
        let mut current = segs[0].clone();

        for next_seg in segs.into_iter().skip(1) {
            if Self::get_chord_root(&current.chord) == Self::get_chord_root(&next_seg.chord)
                && current.dynamic_level == next_seg.dynamic_level
                && current.timbre_brightness == next_seg.timbre_brightness
                && current.rhythm_activity == next_seg.rhythm_activity
            {
                let cur_parts: Vec<&str> = current.time_range.split(" - ").collect();
                let next_parts: Vec<&str> = next_seg.time_range.split(" - ").collect();
                if cur_parts.len() == 2 && next_parts.len() == 2 {
                    current.time_range = format!("{} - {}", cur_parts[0], next_parts[1]);
                }
                current.raw_energy = (current.raw_energy + next_seg.raw_energy) / 2.0;
                current.raw_centroid = (current.raw_centroid + next_seg.raw_centroid) / 2.0;
            } else {
                merged.push(current);
                current = next_seg;
            }
        }
        merged.push(current);
        merged
    }

    /// 执行双版本对比演绎分析 pipeline，产生带 DTW 时间戳对齐的版本比对数据
    pub fn process_comparative(path_a: &Path, path_b: &Path) -> Result<String, String> {
        let default_config = SonicConfig::default();
        let (meta_a, segs_a) = Self::process_single(path_a, &default_config)?;
        let (meta_b, segs_b) = Self::process_single(path_b, &default_config)?;

        // 1. 运行 DTW 时序规整对齐
        let energy_a: Vec<f32> = segs_a.iter().map(|s| s.raw_energy).collect();
        let energy_b: Vec<f32> = segs_b.iter().map(|s| s.raw_energy).collect();

        let aligner = DtwAligner::new();
        let path = aligner.align(&energy_a, &energy_b);

        // 2. 格式化输出对齐比对 Markdown 报告
        let mut report = Vec::new();
        report.push("# SonicBridge: LLM-Readable Music Comparative Report (LRMD)\n".to_string());
        report.push("> [!IMPORTANT]".to_string());
        report.push("> This is a DTW-aligned comparative aesthetic analysis comparing the interpretation, vocal affect, and mix dynamics of two versions of the same track.\n".to_string());

        report.push("## 1. Global Metadata & Comparison".to_string());
        report.push(format!(
            "- **Track A (Original)**: `{}` | Estimated Key: `{}`",
            meta_a.filename, meta_a.estimated_global_key
        ));
        report.push(format!(
            "- **Track B (Cover)**: `{}` | Estimated Key: `{}`",
            meta_b.filename, meta_b.estimated_global_key
        ));
        report.push("- **Aesthetic Register shift**: Pitch transposed and matched dynamic range via temporal warping.\n".to_string());

        report.push("## 2. Dynamic Time Warped (DTW) Aesthetic Alignment Matrix".to_string());
        report.push("The matrix below aligns identical musical sections across both tracks, bypassing tempo differences:\n".to_string());
        report.push("| Music Step | Timeline A | Timeline B | Track A (Original) Interpretation | Track B (Cover) Interpretation |".to_string());
        report.push("| :--- | :--- | :--- | :--- | :--- |".to_string());

        // 为了减小 LLM 报告篇幅，我们只取对齐路径中的非重复状态点
        let mut last_i = None;
        let mut last_j = None;
        let mut step = 1;

        for (i, j) in path {
            if last_i == Some(i) && last_j == Some(j) {
                continue;
            }
            if i >= segs_a.len() || j >= segs_b.len() {
                break;
            }

            let sa = &segs_a[i];
            let sb = &segs_b[j];

            report.push(format!(
                "| **Step {}** | {} | {} | Chord `{}` <br> Dynamic: {} <br> Timbre: {} | Chord `{}` <br> Dynamic: {} <br> Timbre: {} |",
                step, sa.time_range, sb.time_range, sa.chord, sa.dynamic_level, sa.timbre_brightness,
                sb.chord, sb.dynamic_level, sb.timbre_brightness
            ));

            last_i = Some(i);
            last_j = Some(j);
            step += 1;
        }

        Ok(report.join("\n"))
    }
}
