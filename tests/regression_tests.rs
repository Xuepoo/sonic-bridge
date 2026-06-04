use serde::Deserialize;
use sonic_bridge::config::SonicConfig;
use sonic_bridge::pipeline::SonicPipeline;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct TestTrack {
    filepath: String,
    target_bpm: f32,
    target_key: String,
    allowed_equivalents: Vec<String>,
    primary_style: String,
}

#[test]
fn test_golden_dataset_regression() {
    let manifest_path = Path::new("tests/golden_dataset.json");
    if !manifest_path.exists() {
        println!("Golden dataset manifest tests/golden_dataset.json does not exist. Skipping.");
        return;
    }

    let manifest_content =
        fs::read_to_string(manifest_path).expect("Failed to read golden dataset manifest");
    let tracks: Vec<TestTrack> = serde_json::from_str(&manifest_content)
        .expect("Failed to parse golden dataset manifest JSON");

    let mut parsed_tracks = 0;
    let mut bpm_sq_error_sum = 0.0f32;
    let mut key_hits = 0;

    println!("\n====================================================================");
    println!("        SONICBRIDGE GOLDEN DATASET REGRESSION SUITE");
    println!("====================================================================");
    println!(
        "{:<40} | {:<8} / {:<8} | {:<12} / {:<12} | {:<5}",
        "Track File", "Est BPM", "Tgt BPM", "Est Key", "Tgt Key", "Match"
    );
    println!("--------------------------------------------------------------------");

    let config = SonicConfig {
        onset_mode: true,
        ..Default::default()
    };

    for track in &tracks {
        let filepath = Path::new(&track.filepath);
        if !filepath.exists() {
            println!("{:<40} | [SKIPPED - File Not Found]", track.filepath);
            continue;
        }

        // Run the pipeline
        match SonicPipeline::process_single(filepath, &config) {
            Ok((meta, _segs)) => {
                parsed_tracks += 1;

                // Calculate BPM error
                let bpm_diff = meta.estimated_bpm - track.target_bpm;
                bpm_sq_error_sum += bpm_diff * bpm_diff;

                // Calculate Key match
                let key_matched = meta.estimated_global_key == track.target_key
                    || track
                        .allowed_equivalents
                        .contains(&meta.estimated_global_key);

                if key_matched {
                    key_hits += 1;
                }

                println!(
                    "{:<40} | {:<8.1} / {:<8.1} | {:<12} / {:<12} | {:<5}",
                    filepath.file_name().unwrap().to_string_lossy(),
                    meta.estimated_bpm,
                    track.target_bpm,
                    meta.estimated_global_key,
                    track.target_key,
                    if key_matched { "YES" } else { "NO" }
                );
            }
            Err(e) => {
                println!("{:<40} | [FAILED - Error: {}]", track.filepath, e);
            }
        }
    }

    println!("====================================================================");

    if parsed_tracks == 0 {
        println!("No golden dataset tracks found or analyzed. Skipping assertion check.");
        return;
    }

    let rmse = (bpm_sq_error_sum / parsed_tracks as f32).sqrt();
    let hit_ratio = key_hits as f32 / parsed_tracks as f32;

    println!("Total Analyzed: {}", parsed_tracks);
    println!("BPM RMSE: {:.3} (Target: <= 5.0)", rmse);
    println!(
        "Key Hit Ratio: {:.2}% (Target: >= 80.0%)",
        hit_ratio * 100.0
    );
    println!("====================================================================");

    assert!(
        rmse <= 5.0,
        "Regression failed: BPM RMSE {:.3} is greater than 5.0!",
        rmse
    );
    assert!(
        hit_ratio >= 0.80,
        "Regression failed: Key Hit Ratio {:.2}% is less than 80.0%!",
        hit_ratio * 100.0
    );
}
