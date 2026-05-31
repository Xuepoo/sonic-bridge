use crate::alignment::dtw::DtwAligner;
use crate::config::SonicConfig;
use crate::decoder::AudioDecoder;
use crate::dsp::spectrogram::StftEngine;
use crate::musicology::chroma::ChordClassifier;
use crate::musicology::key::KeyDetector;
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

        // 3. 计算自适应全局 BPM (BPM Estimator based on Multi-Histogram Confidence Voting)
        let frame_duration = hop_size as f32 / sr;

        let bpm_detector = crate::dsp::onset::OnsetDetector::new(config.onset_threshold);
        let bpm_boundaries = bpm_detector.detect_boundaries(&spectrogram);

        // 构建 IOI (Inter-Onset Interval) 直方图桶进行投票
        let mut ioi_histogram = [0.0f32; 41];
        if !bpm_boundaries.is_empty() {
            let boundary_times: Vec<f32> = bpm_boundaries
                .iter()
                .map(|&f| f as f32 * frame_duration)
                .collect();
            for i in 0..boundary_times.len() {
                // 相邻间隔
                if i + 1 < boundary_times.len() {
                    let diff = boundary_times[i + 1] - boundary_times[i];
                    if diff > 0.15 && diff < 2.5 {
                        let b = 60.0 / diff;
                        if (50.0..=250.0).contains(&b) {
                            let bin_idx = (((b - 50.0) / 5.0).floor() as usize).min(40);
                            ioi_histogram[bin_idx] += 1.0;
                        }
                    }
                }
                // 跨拍间隔
                if i + 2 < boundary_times.len() {
                    let diff = boundary_times[i + 2] - boundary_times[i];
                    if diff > 0.15 && diff < 2.5 {
                        let b = 60.0 / diff;
                        if (50.0..=250.0).contains(&b) {
                            let bin_idx = (((b - 50.0) / 5.0).floor() as usize).min(40);
                            ioi_histogram[bin_idx] += 0.5; // 跨拍作为辅助特征
                        }
                    }
                }
            }
        }

        // 对直方图进行一阶高斯平滑
        let mut smooth_hist = [0.0f32; 41];
        for idx in 0..41 {
            let left = if idx > 0 { ioi_histogram[idx - 1] } else { 0.0 };
            let right = if idx + 1 < 41 {
                ioi_histogram[idx + 1]
            } else {
                0.0
            };
            smooth_hist[idx] = left * 0.25 + ioi_histogram[idx] * 0.5 + right * 0.25;
        }

        // 提取能量包络并做移动平均平滑，以提高长周期（慢速 tempo）自相关的稳健性
        let raw_envelope: Vec<f32> = spectrogram
            .iter()
            .map(|frame| {
                let sum_power: f32 = frame.iter().map(|&x| x * x).sum();
                (sum_power / frame.len() as f32).sqrt()
            })
            .collect();

        let mut envelope = vec![0.0f32; raw_envelope.len()];
        #[allow(clippy::needless_range_loop)]
        for i in 0..raw_envelope.len() {
            let start = i.saturating_sub(1);
            let end = (i + 1).min(raw_envelope.len() - 1);
            let sum: f32 = raw_envelope[start..=end].iter().sum();
            envelope[i] = sum / (end - start + 1) as f32;
        }

        let estimated_bpm = if envelope.len() >= 64 {
            let mean: f32 = envelope.iter().sum::<f32>() / envelope.len() as f32;
            let zero_mean_env: Vec<f32> = envelope.iter().map(|&x| x - mean).collect();

            // Lag 范围 60 - 200 BPM -> 1.0s 到 0.3s
            let max_lag = (1.25 / frame_duration).round() as usize; // 下限可到 48 BPM
            let min_lag = (0.28 / frame_duration).round() as usize; // 上限可到 214 BPM

            let mut corr_values = vec![0.0f32; max_lag + 1];
            let mut max_corr = -1e9f32;

            #[allow(clippy::needless_range_loop)]
            for lag in min_lag..=max_lag {
                let mut sum = 0.0f32;
                let mut count = 0;
                for i in 0..(zero_mean_env.len() - lag) {
                    sum += zero_mean_env[i] * zero_mean_env[i + lag];
                    count += 1;
                }
                if count > 0 {
                    let corr = sum / count as f32;
                    corr_values[lag] = corr;
                    if corr > max_corr {
                        max_corr = corr;
                    }
                }
            }

            // 归一化自相关系数以用于加权打分
            let mut corr_norm = vec![0.0f32; max_lag + 1];
            if max_corr > 1e-4 {
                #[allow(clippy::needless_range_loop)]
                for lag in min_lag..=max_lag {
                    corr_norm[lag] = (corr_values[lag] / max_corr).max(0.0);
                }
            }

            // 置信度模型投票决策
            let mut best_lag = 0;
            let mut max_confidence = -1e9f32;

            #[allow(clippy::needless_range_loop)]
            for lag in min_lag..=max_lag {
                let bpm_cand = 60.0 / (lag as f32 * frame_duration);
                let bin_idx = if (50.0..=250.0).contains(&bpm_cand) {
                    (((bpm_cand - 50.0) / 5.0).floor() as usize).min(40)
                } else {
                    40
                };
                let hist_vote = smooth_hist[bin_idx];

                // 置信度公式：自相关值 * (1.0 + 1.2 * 直方图平滑因子)
                let mut confidence = corr_norm[lag] * (1.0 + 1.2 * hist_vote);

                // 人性化舒适节奏偏好曲线 (log-Gaussian centered at 115 BPM, width ln(2))
                let bpm_ratio = bpm_cand / 115.0;
                let log_ratio = bpm_ratio.ln();
                let comfort_weight = (-0.5 * (log_ratio / std::f32::consts::LN_2).powi(2)).exp();
                confidence *= comfort_weight;

                if confidence > max_confidence {
                    max_confidence = confidence;
                    best_lag = lag;
                }
            }

            let mut best_bpm = 60.0 / (best_lag as f32 * frame_duration);

            // 八度谐波判定器 (Octave Harmonic Evaluator)：
            // 如果在 2x lag 处（代表半速 BPM）同样具有较强的自相关，且直方图没有压倒性支持快速倍频，说明实际速度应该是半速！
            let double_lag = best_lag * 2;
            if double_lag <= max_lag && corr_norm[double_lag] > 0.45 * corr_norm[best_lag] {
                let best_bpm_val = 60.0 / (best_lag as f32 * frame_duration);
                let half_bpm_val = best_bpm_val / 2.0;
                let bin_idx_best = if (50.0..=250.0).contains(&best_bpm_val) {
                    (((best_bpm_val - 50.0) / 5.0).floor() as usize).min(40)
                } else {
                    40
                };
                let bin_idx_half = if (50.0..=250.0).contains(&half_bpm_val) {
                    (((half_bpm_val - 50.0) / 5.0).floor() as usize).min(40)
                } else {
                    40
                };

                // 如果快速倍频的直方图投票数没有达到慢速半频的 2.5 倍以上，则安全下折为慢速 BPM
                if smooth_hist[bin_idx_best] < 2.5 * smooth_hist[bin_idx_half] {
                    best_bpm /= 2.0;
                }
            }

            // 最终锁定健康节奏区间
            while best_bpm < 60.0 {
                best_bpm *= 2.0;
            }
            while best_bpm > 200.0 {
                best_bpm /= 2.0;
            }

            best_bpm
        } else {
            120.0f32 // Fallback BPM
        };

        // 自适应节奏主观体感映射 (tempo_feeling)
        let tempo_desc = if estimated_bpm < 75.0 {
            "Slow & Solemn (Adagio/Lento)"
        } else if estimated_bpm < 105.0 {
            "Moderate & Gentle (Andante)"
        } else if estimated_bpm < 135.0 {
            "Moderate & Flowing (Moderato)"
        } else if estimated_bpm < 165.0 {
            "Fast & Energetic (Allegro)"
        } else {
            "Extremely Rapid (Presto)"
        };

        // 4. 初始化乐理分析器与全局 Chroma 累加器
        let chord_classifier = ChordClassifier::new();
        let mut segments = Vec::new();
        let mut global_chroma = vec![0.0f32; 12];

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
        } else if config.beat_mode {
            let beat_duration = 60.0 / estimated_bpm;
            let frames_per_beat = (beat_duration / frame_duration).round() as usize;
            let mut temp = vec![0];
            let mut current_frame = frames_per_beat;
            while current_frame < spectrogram.len() {
                temp.push(current_frame);
                current_frame += frames_per_beat;
            }
            temp.push(spectrogram.len());
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
            let end_frame = if config.onset_mode || config.beat_mode {
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
            let mut sum_flatness = 0.0f32;
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

                // 频谱平坦度 (Spectral Flatness) 计算，量化稀疏极简编曲
                let mut sum_log = 0.0f64;
                let mut sum_val = 0.0f32;
                let eps = 1e-7f64;
                for &mag in frame {
                    sum_log += (mag as f64 + eps).ln();
                    sum_val += mag;
                }
                let mean_log = sum_log / frame.len() as f64;
                let geom_mean = mean_log.exp() as f32;
                let arith_mean = sum_val / frame.len() as f32;
                let flatness = if arith_mean > 0.0 {
                    geom_mean / arith_mean
                } else {
                    0.0
                };
                sum_flatness += flatness;

                // 物理旋律音高追踪器 (Melodic Pitch Tracker)
                let bin_300 = (300.0 / (sr / window_size as f32)).round() as usize; // ~14
                let bin_1200 = (1200.0 / (sr / window_size as f32)).round() as usize; // ~56
                let mut max_mag = 0.0f32;
                let mut melody_bin = 0;
                let mut sum_mag_in_band = 0.0f32;
                #[allow(clippy::needless_range_loop)]
                for bin in bin_300..=bin_1200 {
                    let mag = frame[bin];
                    sum_mag_in_band += mag;
                    if mag > max_mag {
                        max_mag = mag;
                        melody_bin = bin;
                    }
                }

                let mean_mag_in_band = sum_mag_in_band / (bin_1200 - bin_300 + 1) as f32;
                let mut melody_pitch_class = -1;

                // 置信度阈值判定：最强音高能量超出平均本底能量的 2.2 倍，方认定为单一纯正的人声/主旋律线条
                if max_mag > mean_mag_in_band * 2.2 && melody_bin > 0 {
                    let melody_freq = melody_bin as f32 * (sr / window_size as f32);
                    let melody_midi = 69.0 + 12.0 * (melody_freq / 440.0).log2();
                    let pc = (melody_midi.round() as i32) % 12;
                    if pc >= 0 {
                        melody_pitch_class = pc;
                    }
                }

                // Apply flatness-based noise suppression to only accumulate chroma from tonal/melodic frames.
                let tonal_weight = if flatness > 0.22 {
                    0.05f32 // Suppress noisy frames (stomps, claps, percussion, silence)
                } else {
                    1.0f32
                };

                // 色度向量合并 (基于 200Hz - 2kHz 审美带通投影，以线性插值消除离散傅里叶频段泄漏，动态旋律加权以消除敲击/共鸣底噪)
                for (bin, &mag) in frame.iter().enumerate() {
                    let freq = bin as f32 * (sr / window_size as f32);
                    if (200.0..=2000.0).contains(&freq) {
                        let midi_pitch = 69.0 + 12.0 * (freq / 440.0).log2();
                        if midi_pitch >= 0.0 {
                            let p_floor = midi_pitch.floor();
                            let p_ceil = midi_pitch.ceil();
                            let w_high = midi_pitch - p_floor;
                            let w_low = 1.0 - w_high;

                            let pc_low = (p_floor as i32) % 12;
                            let pc_high = (p_ceil as i32) % 12;

                            if pc_low >= 0 && pc_high >= 0 {
                                // Dynamic melody boost and noise gate
                                let weight_low = if melody_pitch_class >= 0 {
                                    if pc_low == melody_pitch_class {
                                        12.0f32
                                    } else {
                                        0.2f32
                                    }
                                } else {
                                    0.3f32
                                };
                                let weight_high = if melody_pitch_class >= 0 {
                                    if pc_high == melody_pitch_class {
                                        12.0f32
                                    } else {
                                        0.2f32
                                    }
                                } else {
                                    0.3f32
                                };

                                sum_chroma[pc_low as usize] +=
                                    mag * w_low * weight_low * tonal_weight;
                                sum_chroma[pc_high as usize] +=
                                    mag * w_high * weight_high * tonal_weight;

                                global_chroma[pc_low as usize] +=
                                    mag * w_low * weight_low * tonal_weight;
                                global_chroma[pc_high as usize] +=
                                    mag * w_high * weight_high * tonal_weight;
                            }
                        }
                    }
                }

                counts += 1;
            }

            let (t_start, t_end) = if config.onset_mode || config.beat_mode {
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
            let mean_flatness = sum_flatness / counts as f32;

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

            // A. 自适应动态电平映射（通过 relative_rms、crest_factor 与 mean_flatness 频谱平坦度联合映射）
            // 稀疏编曲（如 We Will Rock You）能量集中少数频段，flatness 显著偏低 (< 0.12)，据此进行动态电平抑制，纠正语义错位
            let relative_rms = mean_rms / global_max_rms;
            let is_sparse = mean_flatness < 0.12f32;

            let dynamic_desc = if relative_rms < 0.01 {
                "Silent/Near-Silent"
            } else if relative_rms < 0.12 {
                "Very Soft (Pianissimo)"
            } else if relative_rms < 0.35 {
                "Soft & Intimate (Piano)"
            } else if relative_rms < 0.65 {
                if is_sparse {
                    "Soft & Intimate (Piano)"
                } else {
                    "Moderately Intense (Mezzo-Forte)"
                }
            } else if relative_rms < 0.85 {
                if is_sparse {
                    "Moderately Intense (Mezzo-Forte)"
                } else {
                    "Loud & Energetic (Forte)"
                }
            } else {
                if crest_factor < 2.5 {
                    if is_sparse {
                        "Moderately Intense (Mezzo-Forte)"
                    } else {
                        "Loud & Dense (Forte)"
                    }
                } else {
                    if is_sparse {
                        "Loud & Energetic (Forte)"
                    } else {
                        "Exploding Intensity (Fortissimo)"
                    }
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
        let key_detector = KeyDetector::new();
        let global_key = key_detector.detect(&global_chroma);

        let global_metadata = GlobalMetadata {
            filename: audio_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            duration_seconds: duration,
            estimated_bpm,
            estimated_global_key: global_key,
            tempo_feeling: tempo_desc.to_string(),
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
