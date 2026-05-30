use std::fs::File;
use std::path::Path;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

pub struct AudioDecoder {
    path: String,
    sample_rate: u32,
}

impl AudioDecoder {
    pub fn new(path: &Path) -> Result<Self, String> {
        Ok(Self {
            path: path.to_str().ok_or("Invalid path")?.to_string(),
            sample_rate: 22050,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn decode(&self) -> Result<Vec<f32>, String> {
        let file = File::open(&self.path).map_err(|e| e.to_string())?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = Path::new(&self.path).extension() {
            if let Some(ext_str) = ext.to_str() {
                hint.with_extension(ext_str);
            }
        }

        let meta_opts: MetadataOptions = Default::default();
        let fmt_opts: FormatOptions = Default::default();

        let mut format = symphonia::default::get_probe()
            .probe(&hint, mss, fmt_opts, meta_opts)
            .map_err(|e| e.to_string())?;

        let track = format
            .default_track(TrackType::Audio)
            .ok_or("No supported audio track found")?;

        let track_id = track.id;

        let audio_params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio().ok())
            .ok_or("No audio codec parameters")?;

        let src_sample_rate = audio_params
            .sample_rate
            .ok_or("Unknown sample rate")?;

        let dec_opts: AudioDecoderOptions = Default::default();
        let mut decoder = symphonia::default::get_codecs()
            .make_audio_decoder(audio_params, &dec_opts)
            .map_err(|e| e.to_string())?;

        let mut raw_samples: Vec<f32> = Vec::new();

        while let Some(packet) = format.next_packet().map_err(|e| e.to_string())? {
            if packet.track_id != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let num_channels = decoded.spec().channels().count();
                    let mut interleaved: Vec<f32> = Default::default();
                    decoded.copy_to_vec_interleaved(&mut interleaved);

                    if num_channels == 1 {
                        raw_samples.extend_from_slice(&interleaved);
                    } else if num_channels > 1 {
                        for frame in interleaved.chunks_exact(num_channels) {
                            let sum: f32 = frame.iter().sum();
                            raw_samples.push(sum / num_channels as f32);
                        }
                    }
                }
                Err(Error::IoError(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(e) => return Err(e.to_string()),
            }
        }

        if raw_samples.is_empty() {
            return Err("No audio samples decoded".to_string());
        }

        let target_rate = self.sample_rate;
        if src_sample_rate == target_rate {
            Ok(raw_samples)
        } else {
            let resampled = resample_linear(&raw_samples, src_sample_rate, target_rate);
            Ok(resampled)
        }
    }
}

fn resample_linear(src: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    let src_len = src.len();
    if src_len == 0 {
        return Vec::new();
    }
    let duration = src_len as f64 / src_rate as f64;
    let dst_len = (duration * dst_rate as f64).round() as usize;
    let mut dst = Vec::with_capacity(dst_len);

    let ratio = src_rate as f64 / dst_rate as f64;
    for i in 0..dst_len {
        let src_idx_exact = i as f64 * ratio;
        let index_floor = src_idx_exact.floor() as usize;
        if index_floor >= src_len - 1 {
            dst.push(src[src_len - 1]);
        } else {
            let index_ceil = index_floor + 1;
            let weight = (src_idx_exact - index_floor as f64) as f32;
            let val = src[index_floor] * (1.0 - weight) + src[index_ceil] * weight;
            dst.push(val);
        }
    }
    dst
}
