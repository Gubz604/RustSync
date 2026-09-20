use std::{env, eprintln, println, writeln};
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::io::{Write, Read};
use std::collections::HashMap;
use sha2::{Digest, Sha256};
use std::fmt::Write as FmtWrite;

#[derive(Debug)]
enum FileState {
    New,
    Modified,
    Unchanged,
    Deleted,
}

struct FileChange {
    path: PathBuf,
    state: FileState,
}

struct FileEntry {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
    hash: String,
}

impl FileEntry {
    fn new(path: PathBuf, size: u64, modified: SystemTime, hash: String) -> Self {
        Self {
            path,
            size,
            modified,
            hash,
        }
    }

    
    fn compare(&self, file: &FileEntry) -> FileState {
        if self.path == file.path && self.size == file.size && self.hash == file.hash {
            return FileState::Unchanged
        }

        FileState::Modified
    }
}

fn main() {
    let state_path = Path::new("rustsync_state.txt");

    // ------------- Collect and Validate arguments -------------

    let args: Vec<String> = env::args().collect();

    if args.len() == 3 {
        println!("Source directory: {}\nBackup destination: {}", args[1], args[2]);  
    } else {
        eprintln!("Usage: rustsync <source_directory> <destination_directory>");
        return;
    }

    let path = Path::new(&args[1]);

    if !path.exists() {
        eprintln!("Error: source path does not exist");
        return;
    }

    if path.is_dir() {
        println!("Source directory is valid");
    } else {
        eprintln!("Error: source path is not a directory");
        return;
    }

    let destination = Path::new(&args[2]);

    if destination.exists() {
        if !destination.is_dir() {
            eprintln!("Error: destination is not a directory");
            return;
        }
    }

    if destination.is_dir() {
        println!("Destination directory is valid\n");
    } else {
        match fs::create_dir_all(destination) {
            Ok(()) => {
                println!("Destination directory successfully created: {}", destination.display());
            },
            Err(err) => {
                eprintln!("Error: {err}");
                return;
            }
        }
    }

    match validate_paths(path, destination) {
        Ok(true) => { println!("Path canonicalization succeeded!")},
        Ok(false) => {
            eprintln!("Destination cannot be inside source directory");
            return;
        },
        Err(err) => {
            eprintln!("Path validation failed: {err}");
            return;
        }
    }
     // ------------- End Validate arguments -------------

    println!("RustSync");
    let previous_scan: Vec<FileEntry> = match load_scan(state_path) {
        Ok(previous) => { previous },
        Err(err) => {
            eprintln!("Error loading previous scan: {err}");
            return;
        }
    };

    let mut current_scan: Vec<FileEntry> = Vec::new();
    match walk_directory(path, path, &previous_scan, &mut current_scan) {
        Ok(()) => {},
        Err(err) => {
            eprintln!("Scan failed: {err}");
            return;
        }
    }
    println!("{} files were discovered\n\n", current_scan.len());

    let changes = compare_scans(&current_scan, &previous_scan);
    print_changes(&changes);

    match backup_files(&changes, path, destination) {
        Ok(()) => {},
        Err(err) => {
            eprintln!("Backup failed: {err}");
            return;
        }
    }

    match save_scan(&current_scan, state_path) {
        Ok(()) => {},
        Err(err) => {
            eprintln!("Save failed: {err}");
            return;
        }
    }
}

fn walk_directory(current_path: &Path, source_root: &Path, previous_files: &[FileEntry], output: &mut Vec<FileEntry>) -> Result<(), std::io::Error> {
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
                    match previous_files.iter().find(|file| (**file).path == relative_path) {
                        Some(previous_file) => {
                            if (previous_file.size == meta_size) && (previous_file.modified == meta_modified) {
                                output.push(FileEntry::new(relative_path.to_path_buf(), meta_size, meta_modified, previous_file.hash.clone()));
                            } else {
                                output.push(FileEntry::new(relative_path.to_path_buf(), meta_size, meta_modified, hash_file(&entry_path)?));
                            }
                        },
                        None => {
                            output.push(FileEntry::new(relative_path.to_path_buf(), meta_size, meta_modified, hash_file(&entry_path)?));
                        }
                    }
                },
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

fn compare_scans(current_files: &[FileEntry], previous_files: &[FileEntry]) -> Vec<FileChange> {
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
                changes.push(FileChange { path: entry.path.clone(), state });
            },
            None => {
                changes.push(FileChange { path: entry.path.clone(), state: FileState::New });
            }
        }
    }

    for entry in previous_files {
        match current_lookup.get(entry.path.as_path()) {
            Some(_) => {},
            None => {
                changes.push(FileChange { path: entry.path.clone(), state: FileState::Deleted });
            }
        }
    }

    changes
}

fn save_scan(files: &[FileEntry], state_path: &Path) -> Result<(), std::io::Error> {
    let mut file = File::create(state_path)?;

    for entry in files {
        match entry.modified.duration_since(UNIX_EPOCH) {
            Ok(duration) => {
                let seconds = duration.as_secs();
                let nanoseconds = duration.subsec_nanos();

                writeln!(file, "{}|{}|{}|{}|{}", entry.path.display(), entry.size, seconds, nanoseconds, entry.hash)?;
            },
            Err(err) => {
                eprintln!("Timestamp error: {err}");
            }
        }
    }


    Ok(())
}

fn load_scan(state_path: &Path) -> Result<Vec<FileEntry>, std::io::Error> {
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

fn should_ignore(path: &Path) -> bool {
    let ignore_list = [
        "target",
        ".git",
        "rustsync_state.txt",
    ];
    let filename_option = path.file_name();

    match filename_option {
        Some(name) => {
            let filename = name.to_string_lossy();
            return ignore_list.contains(&filename.as_ref())
        },
        None => {
            return false;
        }
    }
}

fn backup_files(changes: &[FileChange], source_root: &Path, backup_root: &Path) -> Result<(), std::io::Error> {
    for change in changes {
        let should_backup: bool = match change.state {
            FileState::Modified => true,
            FileState::New => true,
            FileState::Deleted => false,
            FileState::Unchanged => false,
        };

        if !should_backup {
            continue;
        }

        let source_file = source_root.join(&change.path);
        let backup_file = backup_root.join(&change.path);

        match backup_file.parent() {
            Some(path) => {
                fs::create_dir_all(path)?;
                let bytes = fs::copy(source_file, backup_file)?; 
                println!("Copied {} ({} bytes)", change.path.display(), bytes);
            },
            None => {}
        }
    }

    Ok(())
}

fn print_changes(changes: &[FileChange]) {
    for change in changes {
        match change.state {
            FileState::Modified => {
                println!("Modified: {}", change.path.display());
            },
            FileState::New => {
                println!("New: {}", change.path.display());
            },
            FileState::Unchanged => {
                println!("Unchanged: {}", change.path.display());
            },
            FileState::Deleted => {
                println!("Deleted: {}", change.path.display());
            },
        }
    }
}

fn validate_paths(source: &Path, destination: &Path) -> Result<bool, std::io::Error> {
    let source_canonicalized = fs::canonicalize(source)?;
    let destination_canonicalized = fs::canonicalize(destination)?;

    Ok(!destination_canonicalized.starts_with(source_canonicalized))
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