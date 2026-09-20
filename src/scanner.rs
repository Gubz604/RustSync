use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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
                            if previous_file.size == meta_size
                                && previous_file.modified == meta_modified
                            {
                                output.push(FileEntry::new(
                                    relative_path.to_path_buf(),
                                    meta_size,
                                    meta_modified,
                                    previous_file.hash.clone(),
                                ));
                            } else {
                                output.push(FileEntry::new(
                                    relative_path.to_path_buf(),
                                    meta_size,
                                    meta_modified,
                                    hash_file(&entry_path)?,
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
            walk_directory(&entry_path, source_root, previous_files, output)?;
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

fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    let mut hasher = Sha256::new();

    let mut file = File::open(path)?;

    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();

    let mut hash_string = String::new();

    for byte in result {
        let _ = write!(hash_string, "{:02x}", byte);
    }

    Ok(hash_string)
}
