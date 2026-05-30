use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct AlrcSegment {
    pub time_offset: Duration,
    pub chord: String,
    pub dynamic: String,
    pub timbre: String,
    pub synesthesia: String,
    pub critique: String,
}

#[derive(Debug, Clone)]
pub struct AlrcDoc {
    pub title: String,
    pub artist: String,
    pub total_duration: Duration,
    pub segments: Vec<AlrcSegment>,
}

fn parse_time(s: &str) -> Option<Duration> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let mins: u64 = parts[0].parse().ok()?;
    let seconds_parts: Vec<&str> = parts[1].split('.').collect();
    if seconds_parts.is_empty() {
        return None;
    }
    let secs: u64 = seconds_parts[0].parse().ok()?;
    let mut millis: u64 = 0;
    if seconds_parts.len() > 1 {
        let ff_str = seconds_parts[1];
        let val: u64 = ff_str.parse().ok()?;
        if ff_str.len() == 2 {
            millis = val * 10;
        } else if ff_str.len() == 3 {
            millis = val;
        }
    }
    Some(Duration::from_millis(mins * 60_000 + secs * 1000 + millis))
}

fn parse_line(line: &str) -> Option<AlrcSegment> {
    if !line.starts_with('[') {
        return None;
    }
    let end_time_idx = line.find(']')?;
    let time_str = &line[1..end_time_idx];
    let time_offset = parse_time(time_str)?;

    let mut remaining = &line[end_time_idx + 1..];

    let mut chord = "Unknown".to_string();
    let mut dynamic = "Medium".to_string();
    let mut timbre = "Warm".to_string();
    let mut synesthesia = String::new();

    // Try to parse [和弦:X | 动态:Y | 音色:Z]
    let clean_rem = remaining.trim_start();
    if clean_rem.starts_with('[') {
        if let Some(close_idx) = clean_rem.find(']') {
            let tuple_str = &clean_rem[1..close_idx];
            let parts: Vec<&str> = tuple_str.split('|').collect();
            for part in parts {
                let kv: Vec<&str> = part.split(':').collect();
                if kv.len() == 2 {
                    let k = kv[0].trim();
                    let v = kv[1].trim().to_string();
                    if k == "和弦" {
                        chord = v;
                    } else if k == "动态" {
                        dynamic = v;
                    } else if k == "音色" {
                        timbre = v;
                    }
                }
            }
            remaining = &clean_rem[close_idx + 1..];
        }
    }

    // Try to parse [Synesthesia: A]
    let clean_rem2 = remaining.trim_start();
    if clean_rem2.starts_with('[') {
        if let Some(close_idx) = clean_rem2.find(']') {
            let syn_str = &clean_rem2[1..close_idx];
            if let Some(stripped) = syn_str.strip_prefix("Synesthesia:") {
                synesthesia = stripped.trim().to_string();
            }
            remaining = &clean_rem2[close_idx + 1..];
        }
    }

    let critique = remaining.trim().to_string();

    Some(AlrcSegment {
        time_offset,
        chord,
        dynamic,
        timbre,
        synesthesia,
        critique,
    })
}

impl AlrcDoc {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut title = "Unknown Title".to_string();
        let mut artist = "Unknown Artist".to_string();
        let mut total_duration = Duration::from_secs(0);
        let mut segments = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with("[ti:") {
                if let Some(end) = line.find(']') {
                    title = line[4..end].to_string();
                }
            } else if line.starts_with("[ar:") {
                if let Some(end) = line.find(']') {
                    artist = line[4..end].to_string();
                }
            } else if line.starts_with("[length:") {
                if let Some(end) = line.find(']') {
                    let len_str = &line[8..end];
                    let parts: Vec<&str> = len_str.split(':').collect();
                    if parts.len() == 2 {
                        let mins: u64 = parts[0].parse().unwrap_or(0);
                        let secs: u64 = parts[1].parse().unwrap_or(0);
                        total_duration = Duration::from_secs(mins * 60 + secs);
                    }
                }
            } else if let Some(seg) = parse_line(line) {
                segments.push(seg);
            }
        }

        segments.sort_by_key(|s| s.time_offset);

        if total_duration.as_secs() == 0 && !segments.is_empty() {
            total_duration = segments.last().unwrap().time_offset + Duration::from_secs(5);
        }

        Ok(AlrcDoc {
            title,
            artist,
            total_duration,
            segments,
        })
    }
}

pub fn run_live_render(alrc_path: &Path) -> std::io::Result<()> {
    let doc = AlrcDoc::load_from_file(alrc_path).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Failed to parse ALRC: {}", e),
        )
    })?;

    println!("\x1b[2J\x1b[1;1H"); // Initial clear screen
    let start_time = Instant::now();

    loop {
        let elapsed = start_time.elapsed();
        if elapsed >= doc.total_duration {
            break;
        }

        let active_seg = doc.segments.iter().rfind(|s| s.time_offset <= elapsed);

        // Flicker-free console redraw using carriage return/home cursor escape
        print!("\x1b[H");

        println!(
            "\x1b[1;36m  ♫ SonicBridge Appreciation Player [Playing: {} - {}]\x1b[0m",
            doc.title, doc.artist
        );
        println!("\x1b[90m  =================================================================================\x1b[0m");

        let elapsed_secs = elapsed.as_secs();
        let total_secs = doc.total_duration.as_secs();
        let time_str = format!(
            "{:02}:{:02}.{:02} / {:02}:{:02}.00",
            elapsed_secs / 60,
            elapsed_secs % 60,
            (elapsed.subsec_millis() / 10) % 100,
            total_secs / 60,
            total_secs % 60
        );

        let progress_ratio = if total_secs > 0 {
            elapsed.as_secs_f64() / doc.total_duration.as_secs_f64()
        } else {
            0.0
        };
        let bar_width = 40;
        let filled_width = (progress_ratio * bar_width as f64).round() as usize;
        let filled_width = std::cmp::min(filled_width, bar_width);

        let mut progress_bar = String::new();
        for _ in 0..filled_width {
            progress_bar.push('━');
        }
        if filled_width < bar_width {
            progress_bar.push('●');
            for _ in (filled_width + 1)..bar_width {
                progress_bar.push('─');
            }
        } else {
            progress_bar.push('━');
        }

        println!(
            "  [Elapsed: \x1b[1m{}\x1b[0m] \x1b[32m{}\x1b[0m",
            time_str, progress_bar
        );
        println!();

        if let Some(seg) = active_seg {
            println!("  \x1b[1;33m[Acoustic]\x1b[0m  Chord: \x1b[1;32m{:6}\x1b[0m  |  Intensity: \x1b[1;35m{:8}\x1b[0m  |  Timbre: \x1b[1;34m{:8}\x1b[0m",
                     seg.chord, seg.dynamic, seg.timbre);
            println!();

            if !seg.synesthesia.is_empty() {
                println!(
                    "  \x1b[1;36m[Visual]\x1b[0m    \x1b[3m[Synesthesia: {}]\x1b[0m",
                    seg.synesthesia
                );
            } else {
                println!("  \x1b[1;36m[Visual]\x1b[0m    \x1b[90m[Synesthesia: Quietly waiting for transient signals...]\x1b[0m");
            }
            println!();

            println!("  \x1b[1;32m>>> Appreciation: {}\x1b[0m", seg.critique);
        } else {
            println!("  \x1b[1;33m[Acoustic]\x1b[0m  Chord: Silent  |  Intensity: Quiet     |  Timbre: Warm");
            println!();
            println!("  \x1b[1;36m[Visual]\x1b[0m    \x1b[90m[Synesthesia: Aligning neural nodes with sound waves...]\x1b[0m");
            println!();
            println!("  \x1b[1;32m>>> Appreciation: Connecting to sonic space. Warmup sequence completed. <<<\x1b[0m");
        }

        println!("\x1b[90m  =================================================================================\x1b[0m");
        println!("  \x1b[90mPress Ctrl+C to terminate the appreciation preview.\x1b[0m");

        std::thread::sleep(Duration::from_millis(80));
    }

    println!("\n  \x1b[1;32m[+] Appreciation preview finished.\x1b[0m");
    Ok(())
}
