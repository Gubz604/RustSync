use std::{env, println};
use std::path::{Path, PathBuf};
use std::fs;

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

    let mut files: Vec<PathBuf> = Vec::new();

    walk_directory(path, &mut files);
    println!("{} files were discovered\nContents", files.len());
    for file in &files {
        println!("{}", file.display());
    }
}

fn walk_directory(path: &Path, output: &mut Vec<PathBuf>) {
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
                                    output.push(entry_path);
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