use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
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

#[derive(Debug, Clone)]
struct LrmdSegment {
    start_time: Duration,
    end_time: Duration,
    chord: String,
    dynamic: String,
    timbre: String,
}

fn parse_time(s: &str) -> Option<Duration> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let seconds_parts: Vec<&str> = parts[1].split('.').collect();
    if seconds_parts.is_empty() {
        return None;
    }
    let mins: u64 = parts[0].parse().ok()?;
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

fn find_lrmd_path(alrc_path: &Path) -> Option<PathBuf> {
    let parent = alrc_path.parent()?;
    let stem = alrc_path.file_stem()?.to_str()?;
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                    if filename.starts_with(stem) && filename.ends_with(".lrmd.md") {
                        return Some(path);
                    }
                }
            }
        }
    }
    None
}

fn try_fill_from_lrmd(alrc_doc: &mut AlrcDoc, lrmd_path: &Path) {
    if let Ok(file) = File::open(lrmd_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            let line = line.trim();
            if line.starts_with("- **Filename**:") {
                if let Some(val_idx) = line.find('`') {
                    if let Some(end_idx) = line[val_idx + 1..].find('`') {
                        let full_val = &line[val_idx + 1..val_idx + 1 + end_idx];
                        let clean_val = Path::new(full_val)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or(full_val);
                        let parts: Vec<&str> = clean_val.split(" - ").collect();
                        if parts.len() == 2 {
                            if alrc_doc.artist == "Unknown Artist" {
                                alrc_doc.artist = parts[0].trim().to_string();
                            }
                            if alrc_doc.title == "Unknown Title" {
                                alrc_doc.title = parts[1].trim().to_string();
                            }
                        } else if alrc_doc.title == "Unknown Title" {
                            alrc_doc.title = clean_val.to_string();
                        }
                    }
                }
            } else if line.starts_with("- **Duration**:") {
                if let Some(val_idx) = line.find('`') {
                    if let Some(end_idx) = line[val_idx + 1..].find('`') {
                        let sec_str = &line[val_idx + 1..val_idx + 1 + end_idx];
                        let parts: Vec<&str> = sec_str.split_whitespace().collect();
                        if !parts.is_empty() {
                            if let Ok(secs) = parts[0].parse::<f64>() {
                                if alrc_doc.total_duration.as_secs() == 0
                                    || alrc_doc.total_duration.as_secs() == 120
                                {
                                    alrc_doc.total_duration = Duration::from_secs_f64(secs);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn parse_lrmd_timeline(time_range: &str) -> Option<(Duration, Duration)> {
    let parts: Vec<&str> = time_range.split(" - ").collect();
    if parts.len() != 2 {
        return None;
    }

    let parse_single = |s: &str| -> Option<Duration> {
        let s = s.trim();
        if s.ends_with('s') {
            let val: f64 = s[0..s.len() - 1].parse().ok()?;
            Some(Duration::from_secs_f64(val))
        } else {
            let sub: Vec<&str> = s.split(':').collect();
            if sub.len() == 2 {
                let mins: u64 = sub[0].parse().ok()?;
                let secs: u64 = sub[1].parse().ok()?;
                Some(Duration::from_secs(mins * 60 + secs))
            } else {
                None
            }
        }
    };

    let start = parse_single(parts[0])?;
    let end = parse_single(parts[1])?;
    Some((start, end))
}

fn load_lrmd_table(lrmd_path: &Path) -> Vec<LrmdSegment> {
    let mut segments = Vec::new();
    if let Ok(file) = File::open(lrmd_path) {
        let reader = BufReader::new(file);
        for line in reader.lines().map_while(Result::ok) {
            let line = line.trim();
            if line.starts_with('|') && line.contains(" - ") {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() >= 6 {
                    let raw_time = parts[1].trim();
                    let clean_time = raw_time.trim_matches('*');
                    if let Some((start, end)) = parse_lrmd_timeline(clean_time) {
                        let chord = parts[2]
                            .trim()
                            .trim_matches('`')
                            .trim_matches('"')
                            .to_string();
                        let dynamic = parts[3].trim().to_string();
                        let timbre = parts[4].trim().to_string();
                        segments.push(LrmdSegment {
                            start_time: start,
                            end_time: end,
                            chord,
                            dynamic,
                            timbre,
                        });
                    }
                }
            }
        }
    }
    segments
}

impl AlrcDoc {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = File::open(&path)?;
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

        let mut doc = AlrcDoc {
            title,
            artist,
            total_duration,
            segments,
        };

        if let Some(lrmd_path) = find_lrmd_path(path.as_ref()) {
            try_fill_from_lrmd(&mut doc, &lrmd_path);
        }

        Ok(doc)
    }
}

pub fn run_live_render(alrc_path: &Path) -> std::io::Result<()> {
    let doc = AlrcDoc::load_from_file(alrc_path).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Failed to parse ALRC: {}", e),
        )
    })?;

    let lrmd_segs = if let Some(lrmd_path) = find_lrmd_path(alrc_path) {
        load_lrmd_table(&lrmd_path)
    } else {
        Vec::new()
    };

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

        let mut active_chord = "Unknown".to_string();
        let mut active_dynamic = "Medium".to_string();
        let mut active_timbre = "Warm".to_string();

        if let Some(seg) = active_seg {
            if seg.chord != "Unknown" {
                active_chord = seg.chord.clone();
                active_dynamic = seg.dynamic.clone();
                active_timbre = seg.timbre.clone();
            } else if let Some(l_seg) = lrmd_segs
                .iter()
                .find(|l| elapsed >= l.start_time && elapsed <= l.end_time)
            {
                active_chord = l_seg.chord.clone();
                active_dynamic = l_seg.dynamic.clone();
                active_timbre = l_seg.timbre.clone();
            }

            println!("  \x1b[1;33m[Acoustic]\x1b[0m  Chord: \x1b[1;32m{:6}\x1b[0m  |  Intensity: \x1b[1;35m{:8}\x1b[0m  |  Timbre: \x1b[1;34m{:8}\x1b[0m",
                     active_chord, active_dynamic, active_timbre);
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
            if let Some(l_seg) = lrmd_segs
                .iter()
                .find(|l| elapsed >= l.start_time && elapsed <= l.end_time)
            {
                active_chord = l_seg.chord.clone();
                active_dynamic = l_seg.dynamic.clone();
                active_timbre = l_seg.timbre.clone();
            }

            println!("  \x1b[1;33m[Acoustic]\x1b[0m  Chord: \x1b[1;32m{:6}\x1b[0m  |  Intensity: \x1b[1;35m{:8}\x1b[0m  |  Timbre: \x1b[1;34m{:8}\x1b[0m",
                     active_chord, active_dynamic, active_timbre);
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
