use std::{env, eprintln, println, writeln};
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::io::Write;

#[derive(Debug)]
enum FileState {
    New,
    Modified,
    Unchanged,
    Deleted,
}

struct FileChange<'a> {
    file: &'a FileEntry,
    state: FileState,
}

struct FileEntry {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

impl FileEntry {
    fn new(path: PathBuf, size: u64, modified: SystemTime) -> Self {
        Self {
            path,
            size,
            modified,
        }
    }

    
    fn compare(&self, file: &FileEntry) -> FileState {
        if self.path == file.path && self.size == file.size && self.modified == file.modified {
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
    match walk_directory(path, path, &mut current_scan) {
        Ok(()) => {},
        Err(err) => {
            eprintln!("Scan failed: {err}");
            return;
        }
    }
    println!("{} files were discovered\n\n", current_scan.len());

    let changes = compare_scans(&current_scan, &previous_scan);
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

fn walk_directory(current_path: &Path, source_root: &Path, output: &mut Vec<FileEntry>) -> Result<(), std::io::Error> {
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
            match entry_path.strip_prefix(source_root) {
                Ok(relative_path) => {
                    output.push(FileEntry::new(relative_path.to_path_buf(), metadata.len(), meta_modified));
                },
                Err(err) => {
                    eprintln!("Strip Prefix Error: {err}");
                }
            }
        } else if file_type.is_dir() {
            walk_directory(&entry_path, source_root, output)?;
        } else {
            println!("{} is not supported", entry_path.display());
            continue;
        }
    } 
    
    Ok(())
}

fn compare_scans<'a>(current_files: &'a [FileEntry], previous_files: &[FileEntry]) -> Vec<FileChange<'a>> {
    let mut changes: Vec<FileChange> = Vec::new();

    for entry in current_files {
        match previous_files.iter().find(|file| (**file).path == entry.path) {
            Some(file) => {
                let state = entry.compare(file);
                changes.push(FileChange { file: entry, state });
            },
            None => {
                changes.push(FileChange { file: entry, state: FileState::New });
            }
        }
    }

    let mut deleted_files_list: Vec<&FileEntry> = Vec::new();
    for entry in previous_files {
        match current_files.iter().find(|file| (**file).path == entry.path) {
            Some(_) => {},
            None => {
                deleted_files_list.push(entry);
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

                writeln!(file, "{}|{}|{}|{}", entry.path.display(), entry.size, seconds, nanoseconds)?;
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
        if parts.len() != 4 {
            continue;
        }

        let path: PathBuf = PathBuf::from(parts[0]);

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

        files.push(FileEntry::new(path, size, system_timestamp));
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

        let source_file = source_root.join(&change.file.path);
        let backup_file = backup_root.join(&change.file.path);

        match backup_file.parent() {
            Some(path) => {
                fs::create_dir_all(path)?;
                let bytes = fs::copy(source_file, backup_file)?; 
                println!("Copied {} ({} bytes)", change.file.path.display(), bytes);
            },
            None => {}
        }
    }

    Ok(())
}