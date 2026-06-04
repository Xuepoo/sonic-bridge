use std::fs;
use std::path::{Path, PathBuf};

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
