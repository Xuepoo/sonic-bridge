use crate::alignment::dtw::DtwAligner;
use crate::config::SonicConfig;
use crate::decoder::AudioDecoder;
use crate::dsp::spectrogram::StftEngine;
use crate::dsp::style::{StyleClassifier, StyleVector};
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
    pub confidence: f32,
    pub primary_style: String,
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

        // 3. 准备特征与包络信息
        let frame_duration = hop_size as f32 / sr;

        let bpm_detector = crate::dsp::onset::OnsetDetector::new(config.onset_threshold);
        let bpm_boundaries = bpm_detector.detect_boundaries(&spectrogram);

        // 构建 IOI 直方图桶进行投票
        let mut ioi_histogram = [0.0f32; 41];
        if !bpm_boundaries.is_empty() {
            let boundary_times: Vec<f32> = bpm_boundaries
                .iter()
                .map(|&f| f as f32 * frame_duration)
                .collect();
            for i in 0..boundary_times.len() {
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
                if i + 2 < boundary_times.len() {
                    let diff = boundary_times[i + 2] - boundary_times[i];
                    if diff > 0.15 && diff < 2.5 {
                        let b = 60.0 / diff;
                        if (50.0..=250.0).contains(&b) {
                            let bin_idx = (((b - 50.0) / 5.0).floor() as usize).min(40);
                            ioi_histogram[bin_idx] += 0.5;
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

        // 提取能量包络并做移动平均平滑
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

        let mut precomputed = PrecomputedFeatures {
            envelope: envelope.clone(),
            diff_variance: 0.0,
            onset_density: bpm_boundaries.len() as f32 / duration.max(1.0),
            smooth_hist,
            frame_duration,
            max_confidence: 0.0,
            best_lag: 0,
            corr_norm: Vec::new(),
            variance: 0.0,
            peak_coeff: 0.0,
        };

        if envelope.len() >= 64 {
            let mean: f32 = envelope.iter().sum::<f32>() / envelope.len() as f32;
            let zero_mean_env: Vec<f32> = envelope.iter().map(|&x| x - mean).collect();

            let max_lag = (1.25 / frame_duration).round() as usize;
            let min_lag = (0.28 / frame_duration).round() as usize;

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

            let mut corr_norm = vec![0.0f32; max_lag + 1];
            if max_corr > 1e-4 {
                #[allow(clippy::needless_range_loop)]
                for lag in min_lag..=max_lag {
                    corr_norm[lag] = (corr_values[lag] / max_corr).max(0.0);
                }
            }

            let variance =
                zero_mean_env.iter().map(|&x| x * x).sum::<f32>() / zero_mean_env.len() as f32;
            let peak_coeff = if variance > 1e-6 {
                (max_corr / variance).max(0.0)
            } else {
                0.0
            };

            let mut best_lag = 0;
            let mut max_confidence = -1e9f32;
            let max_vote = smooth_hist.iter().copied().fold(0.0f32, f32::max).max(1.0);

            #[allow(clippy::needless_range_loop)]
            for lag in min_lag..=max_lag {
                let bpm_cand = 60.0 / (lag as f32 * frame_duration);
                let bin_idx = if (50.0..=250.0).contains(&bpm_cand) {
                    (((bpm_cand - 50.0) / 5.0).floor() as usize).min(40)
                } else {
                    40
                };
                let hist_vote = smooth_hist[bin_idx] / max_vote;
                let mut confidence = corr_norm[lag] * (1.0 + 1.2 * hist_vote) * peak_coeff;

                let bpm_ratio = bpm_cand / 115.0;
                let log_ratio = bpm_ratio.ln();
                let comfort_weight = (-0.5 * (log_ratio / std::f32::consts::LN_2).powi(2)).exp();
                confidence *= comfort_weight;

                if confidence > max_confidence {
                    max_confidence = confidence;
                    best_lag = lag;
                }
            }

            let mut diff_sum = 0.0f32;
            for i in 1..envelope.len() {
                diff_sum += (envelope[i] - envelope[i - 1]).powi(2);
            }
            let diff_variance = diff_sum / (envelope.len() - 1) as f32;

            precomputed.diff_variance = diff_variance;
            precomputed.max_confidence = max_confidence;
            precomputed.best_lag = best_lag;
            precomputed.corr_norm = corr_norm;
            precomputed.variance = variance;
            precomputed.peak_coeff = peak_coeff;
        }

        // 临时构造粗略色度统计以辅助风格估计
        let mut temp_chroma = [0.0f32; 12];
        for frame in &spectrogram {
            for (bin, &mag) in frame.iter().enumerate() {
                let freq = bin as f32 * (sr / window_size as f32);
                if (200.0..=2000.0).contains(&freq) {
                    let midi_pitch = 69.0 + 12.0 * (freq / 440.0).log2();
                    if midi_pitch >= 0.0 {
                        let pc = (midi_pitch.round() as i32) % 12;
                        if pc >= 0 {
                            temp_chroma[pc as usize] += mag;
                        }
                    }
                }
            }
        }

        // 4. 运行风格分类器与多检测决策引擎 (BPM 估计部分)
        let style_vector = StyleClassifier::classify(
            &spectrogram,
            &temp_chroma,
            precomputed.onset_density,
            precomputed.diff_variance,
            precomputed.max_confidence,
            sr,
        );

        let selector = EnsembleSelector::new();
        let (estimated_bpm, _, confidence) =
            selector.select(&temp_chroma, &spectrogram, &style_vector, &precomputed);

        // 自适应节奏主观体感映射 (tempo_feeling)
        let tempo_desc = if estimated_bpm < 0.0 {
            "Free Rhythm (Ambient/Rubato)"
        } else if estimated_bpm < 75.0 {
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

        // 5. 初始化乐理分析器与全局 Chroma 累加器
        let chord_classifier = ChordClassifier::new();
        let mut segments = Vec::new();
        let mut global_chroma = [0.0f32; 12];

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
            let active_bpm = if estimated_bpm > 0.0 {
                estimated_bpm
            } else {
                120.0
            };
            let beat_duration = 60.0 / active_bpm;
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

        // 预扫描计算全局最大 RMS 动态，以供局部相对 RMS 分类使用
        let mut global_max_rms = 0.0f32;
        for frame in &spectrogram {
            let sum_power: f32 = frame.iter().map(|&x| x * x).sum();
            let rms = (sum_power / frame.len() as f32).sqrt();
            if rms > global_max_rms {
                global_max_rms = rms;
            }
        }
        if global_max_rms < 0.0001 {
            global_max_rms = 1.0;
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

            let mut sum_rms = 0.0f32;
            let mut sum_centroid = 0.0f32;
            let mut sum_flatness = 0.0f32;
            let mut sum_chroma = vec![0.0f32; 12];
            let mut counts = 0;

            for frame in sub_specs {
                let sum_power: f32 = frame.iter().map(|&x| x * x).sum();
                let rms = (sum_power / frame.len() as f32).sqrt();
                sum_rms += rms;

                // 谱质心
                let mut num = 0.0f32;
                let mut den = 0.0f32;
                for (bin, &mag) in frame.iter().enumerate() {
                    let freq = bin as f32 * (sr / window_size as f32);
                    num += freq * mag;
                    den += mag;
                }
                let centroid = if den > 0.0 { num / den } else { 0.0 };
                sum_centroid += centroid;

                // 频谱平坦度计算
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
                let bin_300 = (300.0 / (sr / window_size as f32)).round() as usize;
                let bin_1200 = (1200.0 / (sr / window_size as f32)).round() as usize;
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

                if max_mag > mean_mag_in_band * 2.2 && melody_bin > 0 {
                    let melody_freq = melody_bin as f32 * (sr / window_size as f32);
                    let melody_midi = 69.0 + 12.0 * (melody_freq / 440.0).log2();
                    let pc = (melody_midi.round() as i32) % 12;
                    if pc >= 0 {
                        melody_pitch_class = pc;
                    }
                }

                let tonal_weight = if flatness > 0.22 { 0.05f32 } else { 1.0f32 };

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

            let rhythm_desc = if relative_rms < 0.02 {
                "Static & Ambient sustained notes"
            } else if relative_rms < 0.25 {
                "Flowing & Legato (Gentle melodic flow)"
            } else {
                "Steady Beat (Clear rhythmic dynamic)"
            };

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

        // 6. 后扫描全局调性属性 (Chroma 充分积累后由 Decision Engine 选择最佳调性)
        let refined_style_vector = StyleClassifier::classify(
            &spectrogram,
            &global_chroma,
            precomputed.onset_density,
            precomputed.diff_variance,
            precomputed.max_confidence,
            sr,
        );

        let (_, global_key, _) = selector.select(
            &global_chroma,
            &spectrogram,
            &refined_style_vector,
            &precomputed,
        );

        // 获取主导音乐风格标签
        let primary_style = if refined_style_vector.ambient_free > 0.40 {
            "Ambient/Ambient Free"
        } else if refined_style_vector.traditional_chinese > 0.40 {
            "Traditional Chinese Folk/Modal"
        } else if refined_style_vector.jazz_rubato > 0.40 {
            "Jazz/Rubato Improvisation"
        } else if style_vector.classical > style_vector.electronic_pop {
            "Western Classical/Acoustic Solo"
        } else {
            "Pop/Rock/Electronic"
        };

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
            confidence,
            primary_style: primary_style.to_string(),
        };

        let merged_segments = Self::merge_segments(segments);
        Ok((global_metadata, merged_segments))
    }

    /// Extracts the fundamental root pitch class from a complex chord symbol
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

    /// Spatiotemporal Chunk Merger
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

    /// 执行双版本对比演绎分析 pipeline
    pub fn process_comparative(path_a: &Path, path_b: &Path) -> Result<String, String> {
        let default_config = SonicConfig::default();
        let (meta_a, segs_a) = Self::process_single(path_a, &default_config)?;
        let (meta_b, segs_b) = Self::process_single(path_b, &default_config)?;

        let energy_a: Vec<f32> = segs_a.iter().map(|s| s.raw_energy).collect();
        let energy_b: Vec<f32> = segs_b.iter().map(|s| s.raw_energy).collect();

        let aligner = DtwAligner::new();
        let path = aligner.align(&energy_a, &energy_b);

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

// =========================================================================
// Multi-Detector and Confidence Voting Ensemble Architecture (Scheme A)
// =========================================================================

pub struct PrecomputedFeatures {
    pub envelope: Vec<f32>,
    pub diff_variance: f32,
    pub onset_density: f32,
    pub smooth_hist: [f32; 41],
    pub frame_duration: f32,
    pub max_confidence: f32,
    pub best_lag: usize,
    pub corr_norm: Vec<f32>,
    pub variance: f32,
    pub peak_coeff: f32,
}

#[derive(Debug, Clone)]
pub struct BpmHypothesis {
    pub bpm: f32,
    pub confidence: f32,
    pub source: &'static str,
}

#[derive(Debug, Clone)]
pub struct KeyHypothesis {
    pub key: String,
    pub confidence: f32,
    pub source: &'static str,
}

#[derive(Debug, Clone)]
pub struct DetectorResult {
    pub bpm_candidates: Vec<BpmHypothesis>,
    pub key_candidates: Vec<KeyHypothesis>,
}

pub trait SpecializedDetector {
    fn name(&self) -> &'static str;
    fn detect(
        &self,
        chroma: &[f32; 12],
        spectrogram: &[Vec<f32>],
        style: &StyleVector,
        features: &PrecomputedFeatures,
    ) -> DetectorResult;
}

pub struct AmbientFreeDetector;
impl SpecializedDetector for AmbientFreeDetector {
    fn name(&self) -> &'static str {
        "Ambient/Free Rhythm Detector"
    }

    fn detect(
        &self,
        _chroma: &[f32; 12],
        _spectrogram: &[Vec<f32>],
        _style: &StyleVector,
        features: &PrecomputedFeatures,
    ) -> DetectorResult {
        let mut bpm_candidates = Vec::new();
        let mut key_candidates = Vec::new();

        if features.onset_density < 0.22
            || features.diff_variance < 0.018
            || features.max_confidence < 0.25
        {
            bpm_candidates.push(BpmHypothesis {
                bpm: -1.0,
                confidence: 0.95,
                source: "AmbientFreeDetector",
            });
            key_candidates.push(KeyHypothesis {
                key: "Silent".to_string(),
                confidence: 0.85,
                source: "AmbientFreeDetector",
            });
        } else {
            bpm_candidates.push(BpmHypothesis {
                bpm: -1.0,
                confidence: 0.05,
                source: "AmbientFreeDetector",
            });
        }

        DetectorResult {
            bpm_candidates,
            key_candidates,
        }
    }
}

pub struct ChinesePentatonicDetector;
impl SpecializedDetector for ChinesePentatonicDetector {
    fn name(&self) -> &'static str {
        "Chinese Pentatonic Detector"
    }

    fn detect(
        &self,
        chroma: &[f32; 12],
        _spectrogram: &[Vec<f32>],
        _style: &StyleVector,
        _features: &PrecomputedFeatures,
    ) -> DetectorResult {
        let mut key_candidates = Vec::new();
        let pitch_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];

        for (idx, &root_name) in pitch_names.iter().enumerate() {
            let root_energy_val = chroma[idx].max(1e-5);
            let gong_missing = chroma[(idx + 5) % 12] + chroma[(idx + 11) % 12];
            let shang_missing = chroma[(idx + 3) % 12] + chroma[(idx + 9) % 12];
            let jiao_missing = chroma[(idx + 1) % 12] + chroma[(idx + 7) % 12];
            let zhi_missing = chroma[(idx + 4) % 12] + chroma[(idx + 10) % 12];
            let yu_missing = chroma[(idx + 2) % 12] + chroma[(idx + 8) % 12];

            let pentatonic_threshold = 0.27 * root_energy_val;

            if gong_missing < pentatonic_threshold {
                key_candidates.push(KeyHypothesis {
                    key: format!("{} 宫调式", root_name),
                    confidence: 0.88 * (1.0 - gong_missing / pentatonic_threshold),
                    source: "ChinesePentatonicDetector",
                });
            }
            if shang_missing < pentatonic_threshold {
                key_candidates.push(KeyHypothesis {
                    key: format!("{} 商调式", root_name),
                    confidence: 0.88 * (1.0 - shang_missing / pentatonic_threshold),
                    source: "ChinesePentatonicDetector",
                });
            }
            if jiao_missing < pentatonic_threshold {
                key_candidates.push(KeyHypothesis {
                    key: format!("{} 角调式", root_name),
                    confidence: 0.88 * (1.0 - jiao_missing / pentatonic_threshold),
                    source: "ChinesePentatonicDetector",
                });
            }
            if zhi_missing < pentatonic_threshold {
                key_candidates.push(KeyHypothesis {
                    key: format!("{} 徵调式", root_name),
                    confidence: 0.88 * (1.0 - zhi_missing / pentatonic_threshold),
                    source: "ChinesePentatonicDetector",
                });
            }
            if yu_missing < pentatonic_threshold {
                key_candidates.push(KeyHypothesis {
                    key: format!("{} 羽调式", root_name),
                    confidence: 0.88 * (1.0 - yu_missing / pentatonic_threshold),
                    source: "ChinesePentatonicDetector",
                });
            }
        }

        DetectorResult {
            bpm_candidates: Vec::new(),
            key_candidates,
        }
    }
}

pub struct WesternClassicalDetector;
impl SpecializedDetector for WesternClassicalDetector {
    fn name(&self) -> &'static str {
        "Western Classical Detector"
    }

    fn detect(
        &self,
        chroma: &[f32; 12],
        _spectrogram: &[Vec<f32>],
        _style: &StyleVector,
        features: &PrecomputedFeatures,
    ) -> DetectorResult {
        let mut bpm_candidates = Vec::new();
        let mut key_candidates = Vec::new();

        if features.best_lag > 0 {
            let best_bpm = 60.0 / (features.best_lag as f32 * features.frame_duration);
            if best_bpm < 75.0 && features.diff_variance < 0.035 {
                bpm_candidates.push(BpmHypothesis {
                    bpm: best_bpm,
                    confidence: 0.90,
                    source: "WesternClassicalDetector",
                });
            } else {
                bpm_candidates.push(BpmHypothesis {
                    bpm: best_bpm,
                    confidence: 0.60,
                    source: "WesternClassicalDetector",
                });
            }
        }

        let pitch_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let kk_major = vec![
            6.35, 2.23, 3.48, 1.50, 4.80, 4.09, 2.52, 5.19, 2.00, 3.66, 2.29, 2.88,
        ];
        let kk_minor = vec![
            6.33, 2.68, 3.52, 6.20, 1.50, 3.53, 2.54, 4.75, 4.80, 2.69, 3.34, 3.17,
        ];

        let mut max_sim = -1.0f32;
        let mut best_key_idx = 0;
        let mut best_is_major = true;

        let chroma_norm = chroma.iter().map(|&x| x * x).sum::<f32>().sqrt().max(1e-5);

        for i in 0..12 {
            let mut maj_shifted = vec![0.0f32; 12];
            for j in 0..12 {
                maj_shifted[j] = kk_major[(j + 12 - i) % 12];
            }
            let maj_norm = maj_shifted.iter().map(|&x| x * x).sum::<f32>().sqrt();
            let dot_maj: f32 = chroma
                .iter()
                .zip(&maj_shifted)
                .map(|(&x, &y)| x * y / maj_norm)
                .sum();
            let sim_maj = dot_maj / chroma_norm;
            if sim_maj > max_sim {
                max_sim = sim_maj;
                best_key_idx = i;
                best_is_major = true;
            }

            let mut min_shifted = vec![0.0f32; 12];
            for j in 0..12 {
                min_shifted[j] = kk_minor[(j + 12 - i) % 12];
            }
            let min_norm = min_shifted.iter().map(|&x| x * x).sum::<f32>().sqrt();
            let dot_min: f32 = chroma
                .iter()
                .zip(&min_shifted)
                .map(|(&x, &y)| x * y / min_norm)
                .sum();
            let sim_min = dot_min / chroma_norm;
            if sim_min > max_sim {
                max_sim = sim_min;
                best_key_idx = i;
                best_is_major = false;
            }
        }

        let mut final_root_idx = best_key_idx;
        let mut final_is_major = best_is_major;

        if !best_is_major {
            let submediant_idx = (best_key_idx + 8) % 12;
            if chroma[submediant_idx] > 1.05 * chroma[best_key_idx] {
                final_root_idx = submediant_idx;
                final_is_major = true;
            }
        }

        let energy_minor_third = chroma[(final_root_idx + 3) % 12];
        let energy_major_third = chroma[(final_root_idx + 4) % 12];
        let root_energy = chroma[final_root_idx];
        let corrected_major_third = (energy_major_third - 0.20 * root_energy).max(0.0);

        if final_is_major {
            if energy_minor_third > 0.88 * corrected_major_third {
                final_is_major = false;
            }
        } else {
            if corrected_major_third > 1.15 * energy_minor_third {
                final_is_major = true;
            }
        }

        let key_name = if final_is_major {
            format!("{} Major", pitch_names[final_root_idx])
        } else {
            format!("{} Minor", pitch_names[final_root_idx])
        };

        key_candidates.push(KeyHypothesis {
            key: key_name,
            confidence: max_sim,
            source: "WesternClassicalDetector",
        });

        DetectorResult {
            bpm_candidates,
            key_candidates,
        }
    }
}

pub struct PopElectronicDetector;
impl SpecializedDetector for PopElectronicDetector {
    fn name(&self) -> &'static str {
        "Pop/Electronic/Rock Detector"
    }

    fn detect(
        &self,
        chroma: &[f32; 12],
        _spectrogram: &[Vec<f32>],
        _style: &StyleVector,
        features: &PrecomputedFeatures,
    ) -> DetectorResult {
        let mut bpm_candidates = Vec::new();
        let mut key_candidates = Vec::new();

        if features.best_lag > 0 {
            let best_bpm = 60.0 / (features.best_lag as f32 * features.frame_duration);

            let double_lag = features.best_lag * 2;
            let mut best_bpm_val = best_bpm;
            if double_lag < features.corr_norm.len()
                && features.corr_norm[double_lag] > 0.45 * features.corr_norm[features.best_lag]
            {
                let half_bpm = best_bpm / 2.0;
                let bin_idx_best = (((best_bpm - 50.0) / 5.0).floor() as usize).min(40);
                let bin_idx_half = (((half_bpm - 50.0) / 5.0).floor() as usize).min(40);
                if features.smooth_hist[bin_idx_best] < 2.5 * features.smooth_hist[bin_idx_half] {
                    best_bpm_val = half_bpm;
                }
            }

            if best_bpm_val < 75.0 && features.diff_variance >= 0.035 {
                let double_bpm = best_bpm_val * 2.0;
                let bin_idx_double = (((double_bpm - 50.0) / 5.0).floor() as usize).min(40);
                if features.smooth_hist[bin_idx_double] > 1.5 {
                    best_bpm_val = double_bpm;
                }
            }

            bpm_candidates.push(BpmHypothesis {
                bpm: best_bpm_val,
                confidence: 0.85,
                source: "PopElectronicDetector",
            });
        }

        let pitch_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let base_major = vec![
            6.47, 2.11, 3.49, 1.90, 4.70, 4.04, 2.51, 5.19, 2.05, 3.68, 2.24, 2.94,
        ];
        let base_minor = vec![
            6.41, 2.64, 3.51, 6.10, 1.55, 3.51, 2.57, 4.77, 4.70, 2.64, 3.27, 3.13,
        ];

        let mut max_sim = -1.0f32;
        let mut best_key_idx = 0;
        let mut best_is_major = true;

        let chroma_norm = chroma.iter().map(|&x| x * x).sum::<f32>().sqrt().max(1e-5);

        for i in 0..12 {
            let mut maj_shifted = vec![0.0f32; 12];
            for j in 0..12 {
                maj_shifted[j] = base_major[(j + 12 - i) % 12];
            }
            let maj_norm = maj_shifted.iter().map(|&x| x * x).sum::<f32>().sqrt();
            let dot_maj: f32 = chroma
                .iter()
                .zip(&maj_shifted)
                .map(|(&x, &y)| x * y / maj_norm)
                .sum();
            let sim_maj = dot_maj / chroma_norm;
            if sim_maj > max_sim {
                max_sim = sim_maj;
                best_key_idx = i;
                best_is_major = true;
            }

            let mut min_shifted = vec![0.0f32; 12];
            for j in 0..12 {
                min_shifted[j] = base_minor[(j + 12 - i) % 12];
            }
            let min_norm = min_shifted.iter().map(|&x| x * x).sum::<f32>().sqrt();
            let dot_min: f32 = chroma
                .iter()
                .zip(&min_shifted)
                .map(|(&x, &y)| x * y / min_norm)
                .sum();
            let sim_min = dot_min / chroma_norm;
            if sim_min > max_sim {
                max_sim = sim_min;
                best_key_idx = i;
                best_is_major = false;
            }
        }

        let final_root_idx = best_key_idx;
        let mut final_is_major = best_is_major;

        let energy_minor_third = chroma[(final_root_idx + 3) % 12];
        let energy_major_third = chroma[(final_root_idx + 4) % 12];
        let root_energy = chroma[final_root_idx];
        let corrected_major_third = (energy_major_third - 0.20 * root_energy).max(0.0);

        if final_is_major {
            if energy_minor_third > 0.88 * corrected_major_third {
                final_is_major = false;
            }
        } else {
            if corrected_major_third > 1.15 * energy_minor_third {
                final_is_major = true;
            }
        }

        let key_name = if final_is_major {
            format!("{} Major", pitch_names[final_root_idx])
        } else {
            format!("{} Minor", pitch_names[final_root_idx])
        };

        key_candidates.push(KeyHypothesis {
            key: key_name,
            confidence: max_sim,
            source: "PopElectronicDetector",
        });

        DetectorResult {
            bpm_candidates,
            key_candidates,
        }
    }
}

pub struct JazzRubatoDetector;
impl SpecializedDetector for JazzRubatoDetector {
    fn name(&self) -> &'static str {
        "Jazz/Rubato/Waltz Detector"
    }

    fn detect(
        &self,
        chroma: &[f32; 12],
        _spectrogram: &[Vec<f32>],
        style: &StyleVector,
        features: &PrecomputedFeatures,
    ) -> DetectorResult {
        let mut bpm_candidates = Vec::new();
        let mut key_candidates = Vec::new();

        if features.best_lag > 0 {
            let best_bpm = 60.0 / (features.best_lag as f32 * features.frame_duration);
            let triple_bpm = best_bpm * 3.0;
            let double_bpm = best_bpm * 2.0;

            let bin_idx_double = (((double_bpm - 50.0) / 5.0).floor() as usize).min(40);
            let bin_idx_triple = (((triple_bpm - 50.0) / 5.0).floor() as usize).min(40);

            let double_vote = features.smooth_hist[bin_idx_double];
            let triple_vote = features.smooth_hist[bin_idx_triple];

            let mut confidence = 0.50;
            let selected_bpm = if style.jazz_rubato > 0.40 || triple_vote > 0.30 * double_vote {
                confidence = 0.85;
                triple_bpm
            } else {
                double_bpm
            };

            bpm_candidates.push(BpmHypothesis {
                bpm: selected_bpm,
                confidence,
                source: "JazzRubatoDetector",
            });
        }

        let pitch_names = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        let temp_major = vec![5.0, 2.0, 3.5, 2.0, 4.5, 4.0, 2.0, 4.5, 2.0, 3.5, 1.5, 4.0];
        let temp_minor = vec![5.0, 2.0, 3.5, 4.5, 2.0, 4.0, 2.0, 4.5, 3.5, 2.0, 1.5, 4.0];

        let mut max_sim = -1.0f32;
        let mut best_key_idx = 0;
        let mut best_is_major = true;

        let chroma_norm = chroma.iter().map(|&x| x * x).sum::<f32>().sqrt().max(1e-5);

        for i in 0..12 {
            let mut maj_shifted = vec![0.0f32; 12];
            for j in 0..12 {
                maj_shifted[j] = temp_major[(j + 12 - i) % 12];
            }
            let maj_norm = maj_shifted.iter().map(|&x| x * x).sum::<f32>().sqrt();
            let dot_maj: f32 = chroma
                .iter()
                .zip(&maj_shifted)
                .map(|(&x, &y)| x * y / maj_norm)
                .sum();
            let sim_maj = dot_maj / chroma_norm;
            if sim_maj > max_sim {
                max_sim = sim_maj;
                best_key_idx = i;
                best_is_major = true;
            }

            let mut min_shifted = vec![0.0f32; 12];
            for j in 0..12 {
                min_shifted[j] = temp_minor[(j + 12 - i) % 12];
            }
            let min_norm = min_shifted.iter().map(|&x| x * x).sum::<f32>().sqrt();
            let dot_min: f32 = chroma
                .iter()
                .zip(&min_shifted)
                .map(|(&x, &y)| x * y / min_norm)
                .sum();
            let sim_min = dot_min / chroma_norm;
            if sim_min > max_sim {
                max_sim = sim_min;
                best_key_idx = i;
                best_is_major = false;
            }
        }

        let key_name = if best_is_major {
            format!("{} Major", pitch_names[best_key_idx])
        } else {
            format!("{} Minor", pitch_names[best_key_idx])
        };

        key_candidates.push(KeyHypothesis {
            key: key_name,
            confidence: max_sim,
            source: "JazzRubatoDetector",
        });

        DetectorResult {
            bpm_candidates,
            key_candidates,
        }
    }
}

pub struct EnsembleSelector {
    detectors: Vec<Box<dyn SpecializedDetector>>,
}

impl Default for EnsembleSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl EnsembleSelector {
    pub fn new() -> Self {
        Self {
            detectors: vec![
                Box::new(AmbientFreeDetector),
                Box::new(ChinesePentatonicDetector),
                Box::new(WesternClassicalDetector),
                Box::new(PopElectronicDetector),
                Box::new(JazzRubatoDetector),
            ],
        }
    }

    pub fn select(
        &self,
        chroma: &[f32; 12],
        spectrogram: &[Vec<f32>],
        style: &StyleVector,
        features: &PrecomputedFeatures,
    ) -> (f32, String, f32) {
        let mut all_bpm: Vec<BpmHypothesis> = Vec::new();
        let mut all_key: Vec<KeyHypothesis> = Vec::new();

        for detector in &self.detectors {
            let res = detector.detect(chroma, spectrogram, style, features);
            all_bpm.extend(res.bpm_candidates);
            all_key.extend(res.key_candidates);
        }

        let mut best_bpm = -1.0;
        let mut max_bpm_weight = -1.0;

        for bpm_cand in &all_bpm {
            let mut style_weight = match bpm_cand.source {
                "AmbientFreeDetector" => style.ambient_free,
                "WesternClassicalDetector" => style.classical,
                "PopElectronicDetector" => style.electronic_pop,
                "JazzRubatoDetector" => style.jazz_rubato,
                _ => 0.20,
            };

            if bpm_cand.bpm > 0.0 {
                let bpm_ratio = bpm_cand.bpm / 115.0;
                let log_ratio = bpm_ratio.ln();
                let comfort_weight = (-0.5 * (log_ratio / std::f32::consts::LN_2).powi(2)).exp();
                style_weight *= comfort_weight;
            }

            let weight = bpm_cand.confidence * style_weight;
            if weight > max_bpm_weight {
                max_bpm_weight = weight;
                best_bpm = bpm_cand.bpm;
            }
        }

        if best_bpm > 0.0 {
            // Waltz 3/4 override: if triple vote is strong and envelope indicates beats,
            // we up-shift to triple tempo (Waltz)
            let best_lag_bpm = 60.0 / (features.best_lag as f32 * features.frame_duration);
            let triple_bpm = best_lag_bpm * 3.0;
            let double_bpm = best_lag_bpm * 2.0;
            let bin_idx_double = (((double_bpm - 50.0) / 5.0).floor() as usize).min(40);
            let bin_idx_triple = (((triple_bpm - 50.0) / 5.0).floor() as usize).min(40);
            let double_vote = features.smooth_hist[bin_idx_double];
            let triple_vote = features.smooth_hist[bin_idx_triple];

            if triple_vote > 0.30 * double_vote
                && triple_vote > 0.8
                && features.diff_variance >= 0.035
            {
                best_bpm = triple_bpm;
            }

            while best_bpm < 60.0 {
                best_bpm *= 2.0;
            }
            while best_bpm > 200.0 {
                best_bpm /= 2.0;
            }
        }

        let mut best_key = "Unknown".to_string();
        let mut max_key_weight = -1.0;
        let mut best_pentatonic_key = None;
        let mut max_pentatonic_conf = -1.0;

        for key_cand in &all_key {
            if key_cand.source == "ChinesePentatonicDetector"
                && key_cand.confidence > max_pentatonic_conf
            {
                max_pentatonic_conf = key_cand.confidence;
                best_pentatonic_key = Some(key_cand.key.clone());
            }

            let style_weight = match key_cand.source {
                "AmbientFreeDetector" => style.ambient_free,
                "ChinesePentatonicDetector" => style.traditional_chinese,
                "WesternClassicalDetector" => style.classical,
                "PopElectronicDetector" => style.electronic_pop,
                "JazzRubatoDetector" => style.jazz_rubato,
                _ => 0.20,
            };

            let weight = key_cand.confidence * style_weight;
            if weight > max_key_weight {
                max_key_weight = weight;
                best_key = key_cand.key.clone();
            }
        }

        // Pentatonic priority override: if pentatonic confidence is high (> 0.50),
        // we select it over Western Major/Minor!
        if max_pentatonic_conf > 0.50 {
            if let Some(pent_key) = best_pentatonic_key {
                best_key = pent_key;
            }
        }

        let joint_confidence = (max_bpm_weight.max(0.0) + max_key_weight.max(0.0)) / 2.0;

        (best_bpm, best_key, joint_confidence)
    }
}
