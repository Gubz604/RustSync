use std::env;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
enum FileState {
    New,
    Modified,
    Unchanged,
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
    println!("RustSync");

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

    let mut files: Vec<FileEntry> = Vec::new();

    walk_directory(path, &mut files);
    println!("{} files were discovered\nContents", files.len());
    for file in &files {
        println!("{} - {} bytes last modified: {:?}", file.path.display(), file.size, file.modified);
    }

    let test_time = SystemTime::now();
    let temp_file_1: FileEntry = FileEntry::new(PathBuf::from("C:\\example\\file.txt"), 1200, test_time);
    let temp_file_2: FileEntry = FileEntry::new(PathBuf::from("C:\\example\\file.txt"), 1200, test_time);
    let temp_file_3: FileEntry = FileEntry::new(PathBuf::from("C:\\example\\file.txt"), 1000, test_time);
    let temp_file_4: FileEntry = FileEntry::new(PathBuf::from("C:\\example\\file.txt"), 1200, test_time + Duration::from_secs(60));

    let test_1: FileState = temp_file_1.compare(&temp_file_2);
    let test_2: FileState = temp_file_1.compare(&temp_file_3);
    let test_3: FileState = temp_file_1.compare(&temp_file_4);

    println!("\n\nTest 1 -- Expected Result: Unchanged");
    match test_1 {
        FileState::Modified => {
            println!("Modified");
        },
        FileState::Unchanged => {
            println!("Unchanged");
        },
        _ => {
            println!("Not Modified or Unchanged");
        }
    }

    println!("Test 2 -- Expected Result: Modified");
    match test_2 {
        FileState::Modified => {
            println!("Modified");
        },
        FileState::Unchanged => {
            println!("Unchanged");
        },
        _ => {
            println!("Not Modified or Unchanged");
        }
    }

    println!("Test 3 -- Expected Result: Modified");
    match test_3 {
        FileState::Modified => {
            println!("Modified");
        },
        FileState::Unchanged => {
            println!("Unchanged");
        },
        _ => {
            println!("Not Modified or Unchanged");
        }
    }

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