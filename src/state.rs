use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use crate::scanner::FileEntry;

pub fn save_scan(files: &[FileEntry], state_path: &Path) -> Result<(), std::io::Error> {
    let mut file = File::create(state_path)?;

    for entry in files {
        match entry.modified.duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let seconds = duration.as_secs();
                let nanoseconds = duration.subsec_nanos();

                writeln!(
                    file,
                    "{}|{}|{}|{}|{}",
                    entry.path.display(),
                    entry.size,
                    seconds,
                    nanoseconds,
                    entry.hash
                )?;
            }
            Err(err) => {
                eprintln!("Timestamp error: {err}");
            }
        }
    }

    Ok(())
}

pub fn load_scan(state_path: &Path) -> Result<Vec<FileEntry>, std::io::Error> {
    let mut files: Vec<FileEntry> = Vec::new();

    if !state_path.exists() {
        return Ok(files);
    }

    let contents = fs::read_to_string(state_path)?;

    for line in contents.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 5 {
            continue;
        }

        let path: PathBuf = PathBuf::from(parts[0]);
        let hash: String = parts[4].to_string();

        let Ok(size) = parts[1].parse::<u64>() else {
            eprintln!("Error retrieving file size during loading: {line}");
            continue;
        };

        let Ok(timestamp_sec) = parts[2].parse::<u64>() else {
            eprintln!("Error getting file modification seconds during loading: {line}");
            continue;
        };

        let Ok(timestamp_nano) = parts[3].parse::<u32>() else {
            eprintln!("Error getting file modification nanoseconds during loading: {line}");
            continue;
        };

        let system_timestamp = UNIX_EPOCH + Duration::new(timestamp_sec, timestamp_nano);

        files.push(FileEntry::new(path, size, system_timestamp, hash));
    }

    Ok(files)
}
