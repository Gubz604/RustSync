use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as FmtWrite;

use crate::scanner::FileEntry;

#[derive(Serialize, Deserialize)]
struct StoredFileEntry {
    path: PathBuf,
    size: u64,
    modified_seconds: u64,
    modified_nanoseconds: u32,
    hash: String,
}

impl StoredFileEntry {
    fn from_file_entry(entry: &FileEntry) -> Result<Self, std::io::Error> {

        let duration = match entry.modified.duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let seconds = duration.as_secs();
                let nanoseconds = duration.subsec_nanos();
                (seconds, nanoseconds)
            }
            Err(err) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid timestamp: {err}")
                ));
            }
        };

        Ok(StoredFileEntry {
            path: entry.path.clone(),
            size: entry.size,
            modified_seconds: duration.0,
            modified_nanoseconds: duration.1,
            hash: entry.hash.clone()
        })
    }

    fn into_file_entry(self) -> Result<FileEntry, std::io::Error> {
        if self.modified_nanoseconds > 999_999_999 {
            return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid timestamp: nanoseconds greater than 999,999,999",
            ));
        }

        let system_timestamp = UNIX_EPOCH + Duration::new(self.modified_seconds, self.modified_nanoseconds);

        Ok(FileEntry::new(
            self.path,
            self.size,
            system_timestamp,
            self.hash
        ))
    }
}

pub fn save_scan(files: &[FileEntry], state_path: &Path) -> Result<(), std::io::Error> {
    let temp = state_path.with_extension("tmp");
    let temp_file = File::create(&temp)?;

    let mut stored_files: Vec<StoredFileEntry> = Vec::new();

    for entry in files {
        stored_files.push(StoredFileEntry::from_file_entry(entry)?);
    }

    serde_json::to_writer_pretty(&temp_file, &stored_files).map_err(|err| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to create JSON when saving file: {err}")
        )
    })?;

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

    let stored_files: Vec<StoredFileEntry> = match serde_json::from_str(&contents) {
        Ok(stored_files) => stored_files,
        Err(err) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to read JSON from file when loading scan: {err}")
            ));
        }
    };

    for file in stored_files {
        files.push(file.into_file_entry()?);
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

    #[test]
    fn load_scan_rejects_corrupted_state() {
        let test_dir = std::env::temp_dir().join("rustsync_load_corrupt_test");
        let state_path = test_dir.join("state.txt");
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir).unwrap();
        }

        fs::create_dir_all(&test_dir).unwrap();

        fs::write(
            &state_path,
        "[
                    {
                        \"path\": \"test.txt\",
                        \"size\": \"NOT_A_NUMBER\",
                        \"modified_seconds\": 123,
                        \"modified_nanoseconds\": 456,
                        \"hash\": \"abc123\"
                    }
                ]"
        ).unwrap();

        let result = load_scan(&state_path);

        assert!(result.is_err());

        let error = result.unwrap_err();

        assert_eq!(
            error.kind(),
            std::io::ErrorKind::InvalidData
        );

        fs::remove_dir_all(&test_dir).unwrap();
    }

    #[test]
    fn load_scan_rejects_invalid_nanoseconds() {
        let test_dir = std::env::temp_dir().join("rustsync_load_invalid_nanoseconds_test");
        let state_path = test_dir.join("state.txt");
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir).unwrap();
        }

        fs::create_dir_all(&test_dir).unwrap();

        fs::write(
            &state_path,
        "[
                    {
                        \"path\": \"test.txt\",
                        \"size\": 10,
                        \"modified_seconds\": 123,
                        \"modified_nanoseconds\": 1000000000,
                        \"hash\": \"abc123\"
                    }
                ]"
        ).unwrap();

        let result = load_scan(&state_path);

        assert!(result.is_err());

        let error = result.unwrap_err();

        assert_eq!(
            error.kind(),
            std::io::ErrorKind::InvalidData
        );

        fs::remove_dir_all(&test_dir).unwrap();
    }

    #[test]
    fn save_and_load_scan_round_trip() {
        let test_dir = std::env::temp_dir().join("rustsync_state_round_trip_test");
        let state_path = test_dir.join("state.json");
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir).unwrap();
        }

        fs::create_dir_all(&test_dir).unwrap();

        let files = vec![FileEntry::new(
            PathBuf::from("folder|name/test.txt"),
            10,
            UNIX_EPOCH + Duration::new(123, 456),
            String::from("this_is_a_hash_1"),
        )];

        save_scan(&files, &state_path).unwrap();
        let loaded = load_scan(&state_path).unwrap();

        assert_eq!(loaded.len(), 1);

        assert_eq!(loaded[0].path, files[0].path);
        assert_eq!(loaded[0].size, files[0].size);
        assert_eq!(loaded[0].modified, files[0].modified);
        assert_eq!(loaded[0].hash, files[0].hash);
        
        fs::remove_dir_all(&test_dir).unwrap();
    }
}