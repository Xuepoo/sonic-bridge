use crate::config::SonicConfig;
use crate::pipeline::SonicPipeline;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub fn scan_directory(
    dir: &Path,
    extensions: &[impl AsRef<str>],
    results: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                scan_directory(&path, extensions, results)?;
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions
                        .iter()
                        .any(|x| x.as_ref().eq_ignore_ascii_case(ext))
                    {
                        results.push(path);
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn get_target_report_path(
    audio_path: &Path,
    batch_dir: &Path,
    out_dir: Option<&str>,
) -> PathBuf {
    match out_dir {
        Some(out) => {
            let rel_path = audio_path.strip_prefix(batch_dir).unwrap_or(audio_path);
            let dest = Path::new(out).join(rel_path);
            PathBuf::from(format!("{}.lrmd.md", dest.display()))
        }
        None => PathBuf::from(format!("{}.lrmd.md", audio_path.display())),
    }
}

#[derive(Debug, Clone)]
pub struct BatchSummary {
    pub total: usize,
    pub processed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub errors: Vec<(PathBuf, String)>,
    pub elapsed: Duration,
}

pub fn run_batch(
    config: &SonicConfig,
    batch_dir: &Path,
    dry_run: bool,
    no_progress: bool,
) -> Result<BatchSummary, String> {
    let start_time = Instant::now();
    let mut all_files = Vec::new();
    scan_directory(batch_dir, &config.extensions, &mut all_files)
        .map_err(|e| format!("Failed to scan directory: {}", e))?;

    let total = all_files.len();
    if dry_run {
        println!("[*] Scanning: {}", batch_dir.display());
        println!("[+] Found {} audio files", total);
        return Ok(BatchSummary {
            total,
            processed: 0,
            skipped: 0,
            failed: 0,
            errors: Vec::new(),
            elapsed: start_time.elapsed(),
        });
    }

    // Filter skip_existing files
    let mut files_to_process = Vec::new();
    let mut skipped = 0;
    for file in all_files {
        let target = get_target_report_path(&file, batch_dir, config.out_dir.as_deref());
        let exists_and_valid =
            target.exists() && fs::metadata(&target).map(|m| m.len() > 0).unwrap_or(false);
        if config.skip_existing && exists_and_valid {
            skipped += 1;
        } else {
            files_to_process.push(file);
        }
    }

    let to_process_count = files_to_process.len();
    if !config.quiet_mode {
        println!("[*] Scanning: {}", batch_dir.display());
        println!("[+] Found {} audio files", total);
        println!("[+] Already analyzed: {} (skipped)", skipped);
        println!("[+] Remaining: {} files", to_process_count);
    }

    if to_process_count == 0 {
        return Ok(BatchSummary {
            total,
            processed: 0,
            skipped,
            failed: 0,
            errors: Vec::new(),
            elapsed: start_time.elapsed(),
        });
    }

    // Queue setup (reverse order for pop)
    files_to_process.reverse();
    let queue = Arc::new(Mutex::new(files_to_process));
    let (tx, rx) = std::sync::mpsc::channel();

    // Concurrency setup
    let jobs = config.jobs.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1)
    });

    for _ in 0..jobs {
        let queue_clone = Arc::clone(&queue);
        let tx_clone = tx.clone();
        let config_clone = config.clone();
        let batch_dir_clone = PathBuf::from(batch_dir);

        std::thread::spawn(move || {
            loop {
                let file_path = {
                    let mut q = queue_clone.lock().unwrap();
                    q.pop()
                };

                let file = match file_path {
                    Some(f) => f,
                    None => break,
                };

                let target = get_target_report_path(
                    &file,
                    &batch_dir_clone,
                    config_clone.out_dir.as_deref(),
                );

                let run_start = Instant::now();
                let result = SonicPipeline::process_single(&file, &config_clone);
                let duration = run_start.elapsed();

                match result {
                    Ok((meta, segs)) => {
                        // Generate report text
                        let is_onset_active = config_clone.onset_mode;
                        let is_beat_active = config_clone.beat_mode;
                        let mut report = Vec::new();
                        report.push(
                            "# SonicBridge: LLM-Readable Music Descriptor (LRMD)\n".to_string(),
                        );
                        report.push("> [!NOTE]".to_string());
                        report.push("> This is a physical-to-semantic acoustic report generated by SonicBridge. Pure-text LLMs can 'listen to' and 'appreciate' this track via this spatiotemporal matrix.\n".to_string());
                        report.push("## 1. Global Acoustic & Musicological Metadata".to_string());
                        report.push(format!("- **Filename**: `{}`", meta.filename));
                        report.push(format!(
                            "- **Duration**: `{:.2} seconds`",
                            meta.duration_seconds
                        ));
                        if meta.estimated_bpm < 0.0 {
                            report.push(format!(
                                "- **Tempo (BPM)**: `Unknown (Ambient / Free Rhythm)` ({})",
                                meta.tempo_feeling
                            ));
                        } else {
                            report.push(format!(
                                "- **Tempo (BPM)**: `{:.1} BPM` ({})",
                                meta.estimated_bpm, meta.tempo_feeling
                            ));
                        }
                        report.push(format!(
                            "- **Estimated Key**: `{}`",
                            meta.estimated_global_key
                        ));
                        report.push(format!("- **Primary Style**: `{}`", meta.primary_style));
                        report.push(format!(
                            "- **Analysis Confidence**: `{:.2}`\n",
                            meta.confidence
                        ));

                        let interval_header = if is_onset_active {
                            "## 2. Spatiotemporal Track Analysis (Adaptive Onset Intervals)"
                        } else if is_beat_active {
                            "## 2. Spatiotemporal Track Analysis (Beat-Synchronous Resampling)"
                        } else {
                            &format!(
                                "## 2. Spatiotemporal Track Analysis ({:.1}-Second Intervals)",
                                config_clone.step_size
                            )
                        };
                        report.push(interval_header.to_string());
                        report.push("| Timeline | Chord | Dynamic Intensity | Timbral Brightness | Rhythmic & Transient Activity |".to_string());
                        report.push("| :--- | :--- | :--- | :--- | :--- |".to_string());

                        for seg in &segs {
                            report.push(format!(
                                "| **{}** | `{}` | {} | {} | {} |",
                                seg.time_range,
                                seg.chord,
                                seg.dynamic_level,
                                seg.timbre_brightness,
                                seg.rhythm_activity
                            ));
                        }

                        let report_text = report.join("\n");

                        // Recreate subdirectory structure if needed
                        if let Some(parent) = target.parent() {
                            let _ = fs::create_dir_all(parent);
                        }

                        match fs::write(&target, report_text) {
                            Ok(_) => {
                                let _ = tx_clone.send((file, Ok(()), duration));
                            }
                            Err(e) => {
                                let _ = tx_clone.send((
                                    file,
                                    Err(format!("Failed to write report: {}", e)),
                                    duration,
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx_clone.send((file, Err(e), duration));
                    }
                }
            }
        });
    }
    drop(tx);

    let mut processed = 0;
    let mut failed = 0;
    let mut errors = Vec::new();
    let is_terminal = std::io::IsTerminal::is_terminal(&std::io::stdout());
    let render_progress = !no_progress && is_terminal && !config.quiet_mode;

    for (file, res, _duration) in rx {
        processed += 1;
        match res {
            Ok(_) => {}
            Err(err_msg) => {
                failed += 1;
                errors.push((file, err_msg));
            }
        }

        if render_progress {
            let percentage = (processed * 100) / to_process_count;
            let elapsed = start_time.elapsed();

            // ETA smoothing: display Calculating... for the first 3 files
            let eta_str = if processed < 3 {
                "Calculating...".to_string()
            } else {
                let avg_sec = elapsed.as_secs_f32() / processed as f32;
                let remaining_sec = (to_process_count - processed) as f32 * avg_sec;
                let m = (remaining_sec / 60.0).floor() as u32;
                let s = (remaining_sec % 60.0).round() as u32;
                format!("{}m {}s", m, s)
            };

            let width = 20;
            let filled = (percentage * width) / 100;
            let mut bar = String::new();
            for i in 0..width {
                if i < filled {
                    bar.push('█');
                } else {
                    bar.push('░');
                }
            }

            print!(
                "\rProcessing [{}] {}/{} ({}%) | {} threads | ETA: {}",
                bar, processed, to_process_count, percentage, jobs, eta_str
            );
            let _ = std::io::stdout().flush();
        }
    }

    if render_progress {
        println!();
    }

    Ok(BatchSummary {
        total,
        processed,
        skipped,
        failed,
        errors,
        elapsed: start_time.elapsed(),
    })
}
