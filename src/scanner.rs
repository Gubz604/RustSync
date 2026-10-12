use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::fs;

use crate::hashing::hash_file;


pub struct FileChange {
    pub path: PathBuf,
    pub state: FileState,
}

#[derive(Debug, PartialEq)]
pub enum FileState {
    New,
    Modified,
    Unchanged,
    Deleted,
}

#[derive(Debug)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub hash: String,
}

impl FileEntry {
    pub fn new(path: PathBuf, size: u64, modified: SystemTime, hash: String) -> Self {
        Self {
            path,
            size,
            modified,
            hash,
        }
    }

    pub fn compare(&self, file: &FileEntry) -> FileState {
        if self.path == file.path && self.size == file.size && self.hash == file.hash {
            return FileState::Unchanged;
        }

        FileState::Modified
    }
}

// --------------------------------- Functions ---------------------------------

pub fn walk_directory(
    current_path: &Path,
    source_root: &Path,
    previous_files: &[FileEntry],
    output: &mut Vec<FileEntry>,
    verify: bool,
) -> Result<(), std::io::Error> {
    let content = fs::read_dir(current_path)?;

    for entry in content {
        let dir_entry = entry?;
        let file_type = dir_entry.file_type()?;
        let entry_path = dir_entry.path();

        if should_ignore(&entry_path) {
            continue;
        }

        if file_type.is_file() {
            let metadata = fs::metadata(&entry_path)?;
            let meta_modified = metadata.modified()?;
            let meta_size = metadata.len();
            match entry_path.strip_prefix(source_root) {
                Ok(relative_path) => {
                    match previous_files
                        .iter()
                        .find(|file| (**file).path == relative_path)
                    {
                        Some(previous_file) => {
                            let metadata_matches =
                                previous_file.size == meta_size
                                    && previous_file.modified == meta_modified;

                            let should_hash = verify || !metadata_matches;
                            if should_hash {
                                output.push(FileEntry::new(
                                    relative_path.to_path_buf(),
                                    meta_size,
                                    meta_modified,
                                    hash_file(&entry_path)?,
                                ));
                            } else {
                                output.push(FileEntry::new(
                                    relative_path.to_path_buf(),
                                    meta_size,
                                    meta_modified,
                                    previous_file.hash.clone(),
                                ));
                            }
                        }
                        None => {
                            output.push(FileEntry::new(
                                relative_path.to_path_buf(),
                                meta_size,
                                meta_modified,
                                hash_file(&entry_path)?,
                            ));
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Strip Prefix Error: {err}");
                }
            }
        } else if file_type.is_dir() {
            walk_directory(&entry_path, source_root, previous_files, output, verify)?;
        } else {
            println!("{} is not supported", entry_path.display());
            continue;
        }
    }

    Ok(())
}

pub fn compare_scans(current_files: &[FileEntry], previous_files: &[FileEntry]) -> Vec<FileChange> {
    let mut changes: Vec<FileChange> = Vec::new();
    let mut previous_lookup: HashMap<&Path, &FileEntry> = HashMap::new();
    let mut current_lookup: HashMap<&Path, &FileEntry> = HashMap::new();

    for entry in previous_files {
        previous_lookup.insert(entry.path.as_path(), entry);
    }

    for entry in current_files {
        current_lookup.insert(entry.path.as_path(), entry);
    }

    for entry in current_files {
        match previous_lookup.get(entry.path.as_path()) {
            Some(file) => {
                let state = entry.compare(file);
                changes.push(FileChange {
                    path: entry.path.clone(),
                    state,
                });
            }
            None => {
                changes.push(FileChange {
                    path: entry.path.clone(),
                    state: FileState::New,
                });
            }
        }
    }

    for entry in previous_files {
        match current_lookup.get(entry.path.as_path()) {
            Some(_) => {}
            None => {
                changes.push(FileChange {
                    path: entry.path.clone(),
                    state: FileState::Deleted,
                });
            }
        }
    }

    changes
}

fn should_ignore(path: &Path) -> bool {
    let ignore_list = ["target", ".git", "rustsync_state.txt"];
    let filename_option = path.file_name();

    match filename_option {
        Some(name) => {
            let filename = name.to_string_lossy();
            return ignore_list.contains(&filename.as_ref());
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
use std::fs;
    use std::path::PathBuf;
    use crate::hashing::hash_file;

    use crate::scanner::{FileEntry, walk_directory};

    #[test]
    fn test_walk_directory_to_hash_with_correct_size_and_modified() {

        let test_dir = std::env::temp_dir().join("walk_directory_correct_hash_and_size");
        let file_path = test_dir.join("test.txt");
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir).unwrap();
        }

        fs::create_dir_all(&test_dir).unwrap();

        fs::write(&file_path, b"hello").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();

        let previous_files: Vec<FileEntry> = vec![
            FileEntry::new(PathBuf::from("test.txt"), metadata.len(), metadata.modified().unwrap(), String::from("wrong_hash"))
        ];

        let mut output: Vec<FileEntry> = Vec::new();

        walk_directory(&test_dir, &test_dir, &previous_files, &mut output, true).unwrap();

        assert_eq!(output.len(), 1);

        assert_ne!(output[0].hash, "wrong_hash");
        assert_eq!(output[0].hash, hash_file(&file_path).unwrap());

        fs::remove_dir_all(&test_dir).unwrap();
    }

    #[test]
    fn walk_directory_reuses_hash_when_verify_disabled() {
        let test_dir = std::env::temp_dir().join("walk_directory_false");
        let file_path = test_dir.join("test.txt");
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir).unwrap();
        }

        fs::create_dir_all(&test_dir).unwrap();

        fs::write(&file_path, b"hello").unwrap();

        let metadata = fs::metadata(&file_path).unwrap();

        let previous_files: Vec<FileEntry> = vec![
            FileEntry::new(PathBuf::from("test.txt"), metadata.len(), metadata.modified().unwrap(), String::from("wrong_hash"))
        ];

        let mut output: Vec<FileEntry> = Vec::new();

        walk_directory(&test_dir, &test_dir, &previous_files, &mut output, false).unwrap();

        assert_eq!(output.len(), 1);

        assert_eq!(output[0].hash, "wrong_hash");

        fs::remove_dir_all(&test_dir).unwrap();
    }
}