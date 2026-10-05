use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use sha2::{Digest, Sha256};
use std::fmt::Write as FmtWrite;

use crate::scanner::FileEntry;

pub fn save_scan(files: &[FileEntry], state_path: &Path) -> Result<(), std::io::Error> {
    let temp = state_path.with_extension("tmp");
    let mut temp_file = File::create(&temp)?;

    for entry in files {
        match entry.modified.duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let seconds = duration.as_secs();
                let nanoseconds = duration.subsec_nanos();

                writeln!(
                    temp_file,
                    "{}|{}|{}|{}|{}",
                    entry.path.display(),
                    entry.size,
                    seconds,
                    nanoseconds,
                    entry.hash
                )?;
            }
            Err(err) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid file timestamp: {err}"),
                ));
            }
        }
    }

    drop(temp_file);
    fs::rename(&temp, state_path)?;

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


fn create_id(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    let result = hasher.finalize();
    
    let mut hash_string = String::new();

    for byte in result {
        let _ = write!(hash_string, "{:02x}", byte);
    }

    hash_string
}

pub fn create_local_destination_id(destination: &Path) -> Result<String, std::io::Error> {
    let canonicalized_destination = fs::canonicalize(destination)?;

    Ok(create_id(&canonicalized_destination.to_string_lossy()))
}

pub fn create_remote_destination_id(server_address: &str) -> String {
    create_id(server_address)
}

pub fn create_job_id(source: &Path) -> Result<String, std::io::Error> {
    let canonicalized_path = fs::canonicalize(source)?;

    Ok(create_id(&canonicalized_path.to_string_lossy()))
}

pub fn create_state_paths(job_id: &str, local_destination_id: &str, remote_destination_id: &str) -> Result<(PathBuf, PathBuf), std::io::Error> {
    let job_dir = PathBuf::from(".rustsync")
        .join("state")
        .join(job_id);

    let local_dir = job_dir.join("local");
    let remote_dir = job_dir.join("remote");

    fs::create_dir_all(&local_dir)?;
    fs::create_dir_all(&remote_dir)?;

    let local_filename = format!("{}.txt", local_destination_id);
    let remote_filename = format!("{}.txt", remote_destination_id);

    let local_path = local_dir.join(local_filename);
    let remote_path = remote_dir.join(remote_filename);

    Ok((local_path, remote_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::assert_eq;
    use std::path::PathBuf;

    #[test]
    fn failed_save_preserves_existing_state() {

        let test_dir = std::env::temp_dir().join("rustsync_save_test");
        let state_path = test_dir.join("state.txt");
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir).unwrap();
        }

        fs::create_dir_all(&test_dir).unwrap();

        fs::write(&state_path, b"ORIGINAL STATE").unwrap();

        let file_entry_vector: Vec<FileEntry> = vec![
            FileEntry::new(PathBuf::from("test.txt"), 14, UNIX_EPOCH - Duration::from_secs(1), String::from("this_is_a_hash"))
        ];

        let result = save_scan(&file_entry_vector, &state_path);

        assert!(result.is_err());

        let contents = fs::read(&state_path).unwrap();

        assert_eq!(contents, b"ORIGINAL STATE");

        fs::remove_dir_all(&test_dir).unwrap();
    }
}