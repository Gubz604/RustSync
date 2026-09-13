use std::{env, eprint, eprintln, println};
use std::path::Path;
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

    let contents = fs::read_dir(path);

    match contents {
        Ok(value) => {
            println!("Contents:");
            for entry in value {
                match entry {
                    Ok(dir_entry) => {
                        println!("{}", dir_entry.file_name().to_string_lossy());
                    },
                    Err(err) => {
                        eprint!("Error: {err}");
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Error: {err}");
            return;
        }
    }
}
