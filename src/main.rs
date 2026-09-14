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

    if args.len() == 2 {
        println!("Source directory: {}", args[1]);  
    } else {
        eprintln!("Usage: rustsync <source_directory>");
        return;
    }

    let path = Path::new(&args[1]);

    if !path.exists() {
        eprintln!("Error: source path does not exist");
        return;
    }

    if path.is_dir() {
        println!("Source directory is valid\n");
    } else {
        eprintln!("Error: source path is not a directory");
        return;
    }
     // ------------- End Validate arguments -------------

    println!("RustSync");
    let previous_scan: Vec<FileEntry> = load_scan(state_path);
    let mut current_scan: Vec<FileEntry> = Vec::new();
    walk_directory(path, &mut current_scan);
    compare_scans(&current_scan, &previous_scan);

    save_scan(&current_scan, state_path);
}

fn walk_directory(path: &Path, output: &mut Vec<FileEntry>) {
    let content = fs::read_dir(path);

    match content {
        Ok(dir) => {
            for entry in dir {
                match entry {
                    Ok(dir_entry) => {
                        let sub_dir = dir_entry.file_type();
                        let entry_path = dir_entry.path();
                        
                        match sub_dir {
                            Ok(file_type) => {
                                if file_type.is_file() {
                                    let metadata = fs::metadata(&entry_path);
                                    match metadata {
                                        Ok(meta) => {
                                            let meta_modified = meta.modified();
                                            match meta_modified {
                                                Ok(modified) => {
                                                    output.push(FileEntry::new(entry_path, meta.len(), modified));
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
                                    walk_directory(&entry_path, output);
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