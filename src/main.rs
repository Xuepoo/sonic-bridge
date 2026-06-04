use sonic_bridge::config::SonicConfig;
use sonic_bridge::pipeline::SonicPipeline;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

fn normalize_path(raw_path: &str) -> PathBuf {
    use unicode_normalization::UnicodeNormalization;
    let p = PathBuf::from(raw_path);
    if p.exists() {
        return p;
    }
    // Try NFC normalization
    let nfc_str: String = raw_path.nfc().collect();
    let nfc_path = PathBuf::from(&nfc_str);
    if nfc_path.exists() {
        return nfc_path;
    }
    // Try NFD normalization
    let nfd_str: String = raw_path.nfd().collect();
    let nfd_path = PathBuf::from(&nfd_str);
    if nfd_path.exists() {
        return nfd_path;
    }
    p
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Global interception for help/version flags before any heavy parsing
    if args
        .iter()
        .any(|arg| arg == "-h" || arg == "--help" || arg == "help")
    {
        print_usage();
        return;
    }
    if args
        .iter()
        .any(|arg| arg == "-v" || arg == "--version" || arg == "version")
    {
        print_version();
        return;
    }

    if args.len() < 2 {
        print_usage();
        return;
    }

    let mut config_path = None;
    let mut use_onset = false;
    let mut use_beat = false;
    let mut use_quiet = false;
    let mut use_render = false;
    let mut custom_threshold = None;
    let mut clean_args = Vec::new();

    // Batch mode variables
    let mut batch_dir = None;
    let mut out_dir_opt = None;
    let mut skip_existing_flag = true;
    let mut force_flag = false;
    let mut custom_jobs = None;
    let mut custom_exts = None;
    let mut is_dry_run = false;
    let mut no_progress_flag = false;

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--config" {
            if i + 1 < args.len() {
                config_path = Some(args[i + 1].clone());
                i += 2;
            } else {
                eprintln!(
                    "\x1b[1;31m[-] Error:\x1b[0m --config option requires a valid file path value."
                );
                print_usage();
                std::process::exit(1);
            }
        } else if args[i] == "--threshold" {
            if i + 1 < args.len() {
                if let Ok(val) = args[i + 1].parse::<f32>() {
                    custom_threshold = Some(val);
                } else {
                    eprintln!(
                        "\x1b[1;31m[-] Error:\x1b[0m --threshold option requires a valid floating number."
                    );
                    std::process::exit(1);
                }
                i += 2;
            } else {
                eprintln!("\x1b[1;31m[-] Error:\x1b[0m --threshold option requires a value.");
                print_usage();
                std::process::exit(1);
            }
        } else if args[i] == "--onset" {
            use_onset = true;
            i += 1;
        } else if args[i] == "--beat" {
            use_beat = true;
            i += 1;
        } else if args[i] == "--quiet" || args[i] == "-q" {
            use_quiet = true;
            i += 1;
        } else if args[i] == "--render" {
            use_render = true;
            i += 1;
        } else if args[i] == "--batch" {
            if i + 1 < args.len() {
                batch_dir = Some(args[i + 1].clone());
                i += 2;
            } else {
                eprintln!(
                    "\x1b[1;31m[-] Error:\x1b[0m --batch option requires a target directory."
                );
                std::process::exit(1);
            }
        } else if args[i] == "--out-dir" {
            if i + 1 < args.len() {
                out_dir_opt = Some(args[i + 1].clone());
                i += 2;
            } else {
                eprintln!("\x1b[1;31m[-] Error:\x1b[0m --out-dir option requires a destination directory.");
                std::process::exit(1);
            }
        } else if args[i] == "--skip-existing" {
            skip_existing_flag = true;
            i += 1;
        } else if args[i] == "--force" {
            force_flag = true;
            skip_existing_flag = false;
            i += 1;
        } else if args[i] == "-j" || args[i] == "--jobs" {
            if i + 1 < args.len() {
                if let Ok(jobs) = args[i + 1].parse::<usize>() {
                    custom_jobs = Some(jobs);
                } else {
                    eprintln!(
                        "\x1b[1;31m[-] Error:\x1b[0m --jobs option requires a valid integer."
                    );
                    std::process::exit(1);
                }
                i += 2;
            } else {
                eprintln!("\x1b[1;31m[-] Error:\x1b[0m --jobs option requires a value.");
                std::process::exit(1);
            }
        } else if args[i] == "--ext" {
            if i + 1 < args.len() {
                custom_exts = Some(args[i + 1].clone());
                i += 2;
            } else {
                eprintln!("\x1b[1;31m[-] Error:\x1b[0m --ext option requires extensions list.");
                std::process::exit(1);
            }
        } else if args[i] == "--dry-run" {
            is_dry_run = true;
            i += 1;
        } else if args[i] == "--no-progress" {
            no_progress_flag = true;
            i += 1;
        } else if args[i].starts_with('-') && !Path::new(&args[i]).exists() {
            eprintln!(
                "\x1b[1;31m[-] Error:\x1b[0m Unknown option: \x1b[33m{}\x1b[0m",
                args[i]
            );
            print_usage();
            std::process::exit(1);
        } else {
            clean_args.push(args[i].clone());
            i += 1;
        }
    }

    if clean_args.is_empty() && batch_dir.is_none() {
        print_usage();
        return;
    }

    // Load SonicConfig
    let mut config = if let Some(ref path_str) = config_path {
        SonicConfig::load_from_file(Path::new(path_str)).unwrap_or_else(|e| {
            eprintln!(
                "\x1b[1;33m[!] Warning:\x1b[0m Failed to load config from {}: {}. Falling back to default.",
                path_str, e
            );
            SonicConfig::default()
        })
    } else {
        SonicConfig::load_or_default()
    };

    // Command-line flag override
    if use_onset {
        config.onset_mode = true;
    }
    if use_beat {
        config.beat_mode = true;
    }
    if use_quiet {
        config.quiet_mode = true;
    }
    if let Some(t) = custom_threshold {
        config.onset_threshold = t;
    }

    // Overrides for Batch configurations
    if let Some(out) = out_dir_opt {
        config.out_dir = Some(out);
    }
    config.skip_existing = skip_existing_flag;
    if force_flag {
        config.force = true;
        config.skip_existing = false;
    }
    if let Some(j) = custom_jobs {
        config.jobs = Some(j);
    }
    if let Some(exts_str) = custom_exts {
        config.extensions = exts_str.split(',').map(|s| s.trim().to_string()).collect();
    }

    if let Some(ref dir_str) = batch_dir {
        let dir_path = normalize_path(dir_str);
        if !dir_path.exists() {
            eprintln!(
                "\x1b[1;31m[-] Error:\x1b[0m Directory does not exist: \x1b[33m{}\x1b[0m",
                dir_path.display()
            );
            std::process::exit(1);
        }

        match sonic_bridge::batch::run_batch(&config, &dir_path, is_dry_run, no_progress_flag) {
            Ok(summary) => {
                if is_dry_run {
                    println!(
                        "[+] Would process {} files (skipping {} existing)",
                        summary.total, summary.skipped
                    );
                    std::process::exit(0);
                }

                if !config.quiet_mode {
                    let avg_dur = if summary.processed > 0 {
                        summary.elapsed.as_secs_f32() / summary.processed as f32
                    } else {
                        0.0
                    };
                    println!();
                    println!(
                        "\x1b[1;32m[+]\x1b[0m Batch complete: {} analyzed, {} skipped, {} errors",
                        summary.processed - summary.failed,
                        summary.skipped,
                        summary.failed
                    );
                    println!(
                        "\x1b[1;32m[+]\x1b[0m Total time: {:.2?} (avg {:.2}s/file)",
                        summary.elapsed, avg_dur
                    );
                }

                if summary.failed > 0 {
                    eprintln!("\n\x1b[1;31m[-] Failed Files Summary:\x1b[0m");
                    for (file, err) in &summary.errors {
                        eprintln!("  * \x1b[33m{}\x1b[0m: {}", file.display(), err);
                    }
                    std::process::exit(1);
                }
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("\x1b[1;31m[-] Batch processing failed:\x1b[0m {}", e);
                std::process::exit(1);
            }
        }
    }

    if clean_args.len() == 1 {
        // 单音轨审美分析模式
        let audio_path = normalize_path(&clean_args[0]);

        // Physical existence check
        if !audio_path.exists() {
            eprintln!(
                "\x1b[1;31m[-] Error:\x1b[0m Audio file does not exist: \x1b[33m{}\x1b[0m",
                audio_path.display()
            );
            std::process::exit(1);
        }

        // Appreciation live scrolling mode
        if use_render {
            let alrc_path = audio_path.with_extension("alrc");
            if !alrc_path.exists() {
                eprintln!(
                    "\x1b[1;31m[-] Error:\x1b[0m Aesthetic Lyrics file (.alrc) not found: \x1b[33m{}\x1b[0m\nPlease compile the .alrc file via sonic-bridge-mcp first.",
                    alrc_path.display()
                );
                std::process::exit(1);
            }
            if let Err(e) = sonic_bridge::renderer::run_live_render(&alrc_path) {
                eprintln!(
                    "\x1b[1;31m[-] Error running terminal renderer:\x1b[0m {}",
                    e
                );
                std::process::exit(1);
            }
            return;
        }

        let is_onset_active = config.onset_mode;
        let is_beat_active = config.beat_mode;
        if !config.quiet_mode {
            println!(
                "\x1b[1;36m[*]\x1b[0m Analyzing single track: \x1b[1m{}\x1b[0m (onset mode: \x1b[32m{}\x1b[0m, beat mode: \x1b[32m{}\x1b[0m, step size: \x1b[32m{:.1}s\x1b[0m) ...",
                audio_path.display(),
                is_onset_active,
                is_beat_active,
                config.step_size
            );
        }

        match SonicPipeline::process_single(&audio_path, &config) {
            Ok((meta, segs)) => {
                let report_text = SonicPipeline::generate_lrmd_report(&meta, &segs, &config);

                // 保存到本地
                let out_path = format!("{}.lrmd.md", audio_path.display());
                if let Ok(mut file) = File::create(&out_path) {
                    let _ = file.write_all(report_text.as_bytes());
                    if !config.quiet_mode {
                        println!(
                            "\x1b[1;32m[+]\x1b[0m LRMD report successfully generated and saved to: \x1b[34m{}\x1b[0m",
                            out_path
                        );
                    }
                }

                if !config.quiet_mode {
                    println!("\n=== GENERATED LRMD REPORT PREVIEW ===");
                    println!("{}", report_text);
                }
            }
            Err(e) => {
                eprintln!("\x1b[1;31m[-] Error processing audio track:\x1b[0m {}", e);
            }
        }
    } else if clean_args.len() == 2 {
        // 双音轨比对模式
        let path_a = normalize_path(&clean_args[0]);
        let path_b = normalize_path(&clean_args[1]);

        let mut has_err = false;
        if !path_a.exists() {
            eprintln!(
                "\x1b[1;31m[-] Error:\x1b[0m Audio Track A does not exist: \x1b[33m{}\x1b[0m",
                path_a.display()
            );
            has_err = true;
        }
        if !path_b.exists() {
            eprintln!(
                "\x1b[1;31m[-] Error:\x1b[0m Audio Track B does not exist: \x1b[33m{}\x1b[0m",
                path_b.display()
            );
            has_err = true;
        }
        if has_err {
            std::process::exit(1);
        }

        if !config.quiet_mode {
            println!(
                "\x1b[1;36m[*]\x1b[0m Running DTW Comparative Analysis:\n  - Track A: \x1b[1m{}\x1b[0m\n  - Track B: \x1b[1m{}\x1b[0m",
                path_a.display(),
                path_b.display()
            );
        }

        match SonicPipeline::process_comparative(&path_a, &path_b) {
            Ok(report_text) => {
                let out_path = format!(
                    "{}_vs_{}.lrmd.md",
                    path_a.file_stem().unwrap().to_str().unwrap(),
                    path_b.file_stem().unwrap().to_str().unwrap()
                );
                if let Ok(mut file) = File::create(&out_path) {
                    let _ = file.write_all(report_text.as_bytes());
                    if !config.quiet_mode {
                        println!(
                            "\x1b[1;32m[+]\x1b[0m Comparative LRMD report generated and saved to: \x1b[34m{}\x1b[0m",
                            out_path
                        );
                    }
                }

                if !config.quiet_mode {
                    println!("\n=== COMPARATIVE REPORT PREVIEW ===");
                    println!("{}", report_text);
                }
            }
            Err(e) => {
                eprintln!(
                    "\x1b[1;31m[-] Error running comparative alignment:\x1b[0m {}",
                    e
                );
            }
        }
    } else {
        print_usage();
    }
}

fn print_usage() {
    println!("\x1b[1;36m      ____              _      ____        _     _             \x1b[0m");
    println!("\x1b[1;36m     / ___|  ___  _ __ (_) ___| __ ) _ __ (_) __| | __ _  ___  \x1b[0m");
    println!(
        "\x1b[1;36m     \\___ \\ / _ \\| '_ \\| |/ __|  _ \\| '__|| |/ _` |/ _` |/ _ \\ \x1b[0m"
    );
    println!("\x1b[1;36m      ___) | (_) | | | | | (__| |_) | |   | | (_| | (_| |  __/ \x1b[0m");
    println!(
        "\x1b[1;36m     |____/ \\___/|_| |_|_|\\___|____/|_|   |_|\\__,_|\\__, |\\___| \x1b[0m"
    );
    println!("\x1b[1;36m                                                   |___/       \x1b[0m");
    println!();
    println!(
        "\x1b[1;32mSonicBridge CLI\x1b[0m - LLM-Readable Acoustic Transformer (\x1b[33mv{}\x1b[0m)",
        env!("CARGO_PKG_VERSION")
    );
    println!("\x1b[90m====================================================================\x1b[0m");
    println!("An ultra-fast physical-to-semantic music aesthetic encoder.");
    println!();
    println!("\x1b[1;33mUSAGE:\x1b[0m");
    println!("  \x1b[1mSingle Track Analysis (Generate LRMD Report):\x1b[0m");
    println!("    sonic-bridge \x1b[32m<path_to_audio>\x1b[0m [options]");
    println!();
    println!("  \x1b[1mComparative Track Alignment (DTW Cross-Matching):\x1b[0m");
    println!("    sonic-bridge \x1b[32m<track_A>\x1b[0m \x1b[32m<track_B>\x1b[0m");
    println!();
    println!("  \x1b[1mBatch Processing (Directory Scan Mode):\x1b[0m");
    println!("    sonic-bridge \x1b[32m--batch <directory>\x1b[0m [options]");
    println!();
    println!("\x1b[1;33mOPTIONS:\x1b[0m");
    println!("  \x1b[32m--onset\x1b[0m          Enable event-driven adaptive interval segmenting (Onset detection)");
    println!("                   (Defaults to fixed step time segmenting if omitted)");
    println!("  \x1b[32m--beat\x1b[0m           Enable beat-synchronous resampling (Beat tracking segmentation)");
    println!("  \x1b[32m--threshold <val>\x1b[0m Overwrite default Onset sensitivity threshold (e.g., 1.5 to filter noise)");
    println!("  \x1b[32m--quiet, -q\x1b[0m      Mute outputting markdown report preview in standard stdout");
    println!("  \x1b[32m--config <path>\x1b[0m  Specify target TOML config file path (Override default XDG paths)");
    println!(
        "  \x1b[32m--render\x1b[0m         Start dynamic real-time terminal appreciation scrolling"
    );
    println!("                   (Requires a matching .alrc file under the same directory)");
    println!();
    println!("  \x1b[1;33mBATCH OPTIONS:\x1b[0m");
    println!("  \x1b[32m--out-dir <dir>\x1b[0m  Output reports to this target destination path");
    println!("  \x1b[32m--skip-existing\x1b[0m  Skip tracks with existing .lrmd.md (default)");
    println!("  \x1b[32m--force\x1b[0m          Force processing and override skip logic");
    println!("  \x1b[32m-j, --jobs <N>\x1b[0m   Number of parallel threads");
    println!("  \x1b[32m--ext <exts>\x1b[0m     Comma-separated extensions to scan");
    println!("  \x1b[32m--dry-run\x1b[0m        List files that would be processed");
    println!("  \x1b[32m--no-progress\x1b[0m    Disable rendering progress bars");
    println!("  \x1b[32m-h, --help\x1b[0m       Show this premium help manual");
    println!("  \x1b[32m-v, --version\x1b[0m    Show current version info");
    println!();
    println!("\x1b[1;33mENVIRONMENT:\x1b[0m");
    println!("  Configuration paths comply with XDG specs (\x1b[34m$XDG_CONFIG_HOME/sonic-bridge/\x1b[0m).");
    println!();
}

fn print_version() {
    println!("sonic-bridge version {}", env!("CARGO_PKG_VERSION"));
}
