use sonic_bridge::decoder::AudioDecoder;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

fn generate_sine_wav(
    path: &Path,
    duration: f32,
    sample_rate: u32,
    freq: f32,
) -> std::io::Result<()> {
    let num_samples = (duration * sample_rate as f32) as usize;
    let mut data = Vec::with_capacity(num_samples * 2);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let val = (2.0 * std::f32::consts::PI * freq * t).sin();
        let sample = (val * 32767.0) as i16;
        data.extend_from_slice(&sample.to_le_bytes());
    }

    let mut file = File::create(path)?;
    let subchunk2_size = data.len() as u32;
    let chunk_size = 36 + subchunk2_size;

    // RIFF header
    file.write_all(b"RIFF")?;
    file.write_all(&chunk_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;

    // fmt subchunk
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // subchunk1_size
    file.write_all(&1u16.to_le_bytes())?; // audio_format = 1 (PCM)
    file.write_all(&1u16.to_le_bytes())?; // num_channels = 1
    file.write_all(&sample_rate.to_le_bytes())?; // sample_rate
    let byte_rate = sample_rate * 2;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?; // block_align = num_channels * bits_per_sample/8
    file.write_all(&16u16.to_le_bytes())?; // bits_per_sample = 16

    // data subchunk
    file.write_all(b"data")?;
    file.write_all(&subchunk2_size.to_le_bytes())?;
    file.write_all(&data)?;

    Ok(())
}

#[test]
fn test_decode_wav_file() {
    let fixtures_dir = Path::new("tests/fixtures");
    if !fixtures_dir.exists() {
        fs::create_dir_all(fixtures_dir).unwrap();
    }

    let wav_path = fixtures_dir.join("mock.wav");
    // Generate a 0.2 second 440Hz sine wave wav file at 44100Hz
    generate_sine_wav(&wav_path, 0.2, 44100, 440.0).unwrap();

    assert!(
        wav_path.exists(),
        "Please ensure tests/fixtures/mock.wav exists"
    );

    let decoder = AudioDecoder::new(&wav_path).unwrap();
    let samples = decoder.decode().unwrap();

    // Original duration is 0.2 seconds.
    // Target sample rate is 22050Hz.
    // So the number of samples should be around 0.2 * 22050 = 4410.
    assert!(!samples.is_empty());
    // Allow a small tolerance for rounding
    let expected_len = (0.2 * 22050.0) as usize;
    assert!((samples.len() as isize - expected_len as isize).abs() <= 5);

    assert_eq!(decoder.sample_rate(), 22050);
}
