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
            }
        }
    }
     // ------------- End Validate arguments -------------

    println!("RustSync");
    let previous_scan: Vec<FileEntry> = load_scan(state_path);
    let mut current_scan: Vec<FileEntry> = Vec::new();
    walk_directory(path, path, &mut current_scan);
    println!("{} files were discovered\n\n", current_scan.len());
    compare_scans(&current_scan, &previous_scan);

    backup_files(&current_scan, path, destination);

    save_scan(&current_scan, state_path);
}

fn walk_directory(current_path: &Path, source_root: &Path, output: &mut Vec<FileEntry>) {
    let content = fs::read_dir(current_path);

    match content {
        Ok(dir) => {
            for entry in dir {
                match entry {
                    Ok(dir_entry) => {
                        let sub_dir = dir_entry.file_type();
                        let entry_path = dir_entry.path();

                        if should_ignore(&entry_path) {
                            continue;
                        }
                        
                        match sub_dir {
                            Ok(file_type) => {
                                if file_type.is_file() {
                                    let metadata = fs::metadata(&entry_path);
                                    match metadata {
                                        Ok(meta) => {
                                            let meta_modified = meta.modified();
                                            match meta_modified {
                                                Ok(modified) => {
                                                    match entry_path.strip_prefix(source_root) {
                                                        Ok(relative_path) => {
                                                            output.push(FileEntry::new(relative_path.to_path_buf(), meta.len(), modified));
                                                        },
                                                        Err(err) => {
                                                            eprintln!("Error: {err}");
                                                        }
                                                    }
                                                },
                                                Err(err) => {
                                                    eprintln!("Error: {err}");
                                                }
                                            }
                                        },
                                        Err(err) => {
                                            eprintln!("Error: {err}");
                                        }
                                    }
                                } else if file_type.is_dir() {
                                    walk_directory(&entry_path, &source_root, output);
                                } else {
                                    println!("{} is not supported", entry_path.display());
                                    continue;
                                }
                            },
                            Err(err) => {
                                eprintln!("Error: {err}");
                            }
                        }
                    },
                    Err(err) => {
                        eprintln!("Error: {err}");
                    }
                }
            } 
        },
        Err(err) => {
            eprintln!("Error: {err}");
            return;
        }
    }
}

fn compare_scans(current_files: &[FileEntry], previous_files: &[FileEntry]) {
    for entry in current_files {
        match previous_files.iter().find(|file| (**file).path == entry.path) {
            Some(file) => {
                let state: String = match entry.compare(file) {
                    FileState::Modified => { String::from("Modified") },
                    FileState::Unchanged => { String::from("Unchanged") },
                    FileState::New => { String::from("New") }, 
                    FileState::Deleted => { String::from("Deleted") },
                };

                println!("{}: {} - {} bytes last modified: {:?}", state, entry.path.display(), entry.size, entry.modified);
            },
            None => {
                println!("New: {}", entry.path.display());
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

    if !deleted_files_list.is_empty() {
        println!("\n\nDeleted Files");
        for item in deleted_files_list {
            println!("Deleted: {}", item.path.display());
        }
    }
}

fn save_scan(files: &[FileEntry], state_path: &Path) {
    let file_result = File::create(state_path);
    match file_result {
        Ok(mut file) => {
            for entry in files {
                let duration_result = entry.modified.duration_since(UNIX_EPOCH);
                match duration_result {
                    Ok(duration) => {
                        let seconds = duration.as_secs();
                        let nanoseconds = duration.subsec_nanos();

                        match writeln!(file, "{}|{}|{}|{}", entry.path.display(), entry.size, seconds, nanoseconds) {
                            Ok(_) => {},
                            Err(err) => {
                                eprintln!("Error: {err}");
                            }
                        }
                    },
                    Err(err) => {
                        eprintln!("Error: {err}");
                    }
                }
            }
        },
        Err(err) => {
            eprintln!("Error: {err}");
        }
    }
}

fn load_scan(state_path: &Path) -> Vec<FileEntry> {
    let mut files: Vec<FileEntry> = Vec::new();

    if !state_path.exists() {
        return files;
    }

    let state_path_string = fs::read_to_string(state_path);
    match state_path_string {
        Ok(contents) => {
            for line in contents.lines() {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() != 4 {
                    continue;
                }

                let path: PathBuf = PathBuf::from(parts[0]);

                let size_result = parts[1].parse::<u64>();
                match size_result {
                    Ok(size) => {
                        let timestamp_sec_result = parts[2].parse::<u64>();
                        match timestamp_sec_result {
                            Ok(timestamp_sec) => {
                                let timestamp_nano_result = parts[3].parse::<u32>();
                                match timestamp_nano_result {
                                    Ok(timestamp_nano) => {
                                        let system_timestamp = UNIX_EPOCH + Duration::new(timestamp_sec, timestamp_nano);

                                        files.push(FileEntry::new(path, size, system_timestamp));
                                    },
                                    Err(err) => {
                                        eprintln!("Error: {err}");
                                    }
                                }
                            },
                            Err(err) => {
                                eprintln!("Error: {err}");
                            }
                        }
                    },
                    Err(err) => {
                        eprintln!("Error: {err}");
                    }
                }
            }
        },
        Err(err) => {
            eprintln!("Error: {err}");
        }
    }

    files
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

fn backup_files(files: &[FileEntry], source_root: &Path, backup_root: &Path) {
    for entry in files {
        let source_file = source_root.join(&entry.path);
        let backup_file = backup_root.join(&entry.path);

        let parent_directory_option = backup_file.parent();
        match parent_directory_option {
            Some(path) => {
                let directory = fs::create_dir_all(path);
                match directory {
                    Ok(()) => {
                        match fs::copy(source_file, backup_file) {
                            Ok(bytes) => {
                                println!("Copied {} ({} bytes)", entry.path.display(), bytes);
                            },
                            Err(err) => {
                                eprintln!("Error: {err}");
                            }
                        }
                    },
                    Err(err) => {
                        eprintln!("Error: {err}");
                    }
                }
            },
            None => {}
        }
    }
}