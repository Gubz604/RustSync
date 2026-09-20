use std::{env, eprintln, println, writeln};
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::time::{Duration, UNIX_EPOCH};
use std::io::Write;

mod scanner;

use scanner::{FileEntry, FileState, FileChange, walk_directory, compare_scans};

fn main() {
    let state_path = Path::new("rustsync_state.txt");

    // ------------- Collect and Validate arguments -------------

    let args: Vec<String> = env::args().collect();
    let dry_run_mode;

    if args.len() == 3 {
        dry_run_mode = false;
    } else if args.len() == 4 && args[3] == "--dry-run"{
        dry_run_mode = true;
    } else {
        eprintln!("Usage: rustsync <source_directory> <destination_directory> [--dry-run]");
        return;
    }

    println!("Source directory: {}\nBackup destination: {}", args[1], args[2]);  

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
    
    if !dry_run_mode {
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
    } else {
        println!("Dry run: no files were copied and state was not updated");
    }
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

#[cfg(test)]
mod tests{
    use std::assert_eq;

use super::*;


    #[test]
    fn test_compare_unchanged() {
        let first = FileEntry::new(PathBuf::from("test"), 10, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash"));
        let second = FileEntry::new(PathBuf::from("test"), 10, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash"));

        let result = first.compare(&second);

        assert_eq!(result, FileState::Unchanged); 
    }

    #[test]
    fn test_compare_modified_size_only() {
        let first = FileEntry::new(PathBuf::from("test"), 20, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash"));
        let second = FileEntry::new(PathBuf::from("test"), 10, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash"));

        let result = first.compare(&second);

        assert_eq!(result, FileState::Modified);
    }

    #[test]
    fn test_compare_unchanged_with_different_timestamp() {
        let first = FileEntry::new(PathBuf::from("test"), 10, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash"));
        let second = FileEntry::new(PathBuf::from("test"), 10, UNIX_EPOCH + Duration::from_secs(2_000_000), String::from("this_is_a_hash"));

        let result = first.compare(&second);

        assert_eq!(result, FileState::Unchanged);
    }

    #[test]
    fn test_compare_modified_hashed() {
        let first = FileEntry::new(PathBuf::from("test"), 10, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash"));
        let second = FileEntry::new(PathBuf::from("test"), 10, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_different_hash"));

        let result = first.compare(&second);

        assert_eq!(result, FileState::Modified);
    }

    #[test]
    fn test_compare_scans_file_unchanged() {
        let previous = vec![
            FileEntry::new(PathBuf::from("test1"), 20, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash_1")),
        ];

        let current = vec![
            FileEntry::new(PathBuf::from("test1"), 20, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash_1")),
        ];

        let changes: Vec<FileChange> = compare_scans(&current, &previous);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, PathBuf::from("test1"));
        assert_eq!(changes[0].state, FileState::Unchanged);
    }

    #[test]
    fn test_compare_scans_file_new() {
        let previous = vec![];

        let current = vec![
            FileEntry::new(PathBuf::from("test1"), 20, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash_1")),
        ];

        let changes: Vec<FileChange> = compare_scans(&current, &previous);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, PathBuf::from("test1"));
        assert_eq!(changes[0].state, FileState::New);
    }

    #[test]
    fn test_compare_scans_file_modified_hashed() {
        let previous = vec![
            FileEntry::new(PathBuf::from("test1"), 10, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash_1")),
        ];

        let current = vec![
            FileEntry::new(PathBuf::from("test1"), 10, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash_2")),
        ];

        let changes: Vec<FileChange> = compare_scans(&current, &previous);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, PathBuf::from("test1"));
        assert_eq!(changes[0].state, FileState::Modified);
    }

    #[test]
    fn test_compare_scans_file_modified_size() {
        let previous = vec![
            FileEntry::new(PathBuf::from("test1"), 10, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash_1")),
        ];

        let current = vec![
            FileEntry::new(PathBuf::from("test1"), 20, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash_1")),
        ];

        let changes: Vec<FileChange> = compare_scans(&current, &previous);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, PathBuf::from("test1"));
        assert_eq!(changes[0].state, FileState::Modified);
    }

    #[test]
    fn test_compare_scans_file_deleted() {
        let previous = vec![
            FileEntry::new(PathBuf::from("test1"), 10, UNIX_EPOCH + Duration::from_secs(1_000_000), String::from("this_is_a_hash_1")),
        ];

        let current = vec![];

        let changes: Vec<FileChange> = compare_scans(&current, &previous);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, PathBuf::from("test1"));
        assert_eq!(changes[0].state, FileState::Deleted);
    }
}