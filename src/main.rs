use std::env;
use std::path::{Path, PathBuf};
use std::fs;

struct FileEntry {
    path: PathBuf,
    size: u64,
}

impl FileEntry {
    fn new(path: PathBuf, size: u64) -> Self {
        Self {
            path,
            size,
        }
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
        println!("{} - {} bytes", file.path.display(), file.size);
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
                                    let     metadata = fs::metadata(&entry_path);
                                    match   metadata {
                                        Ok(meta) => {
                                            output.push(FileEntry::new(entry_path, meta.len()));
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