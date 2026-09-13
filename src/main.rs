use std::{env, eprint, println};
use std::path::Path;

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
        eprint!("Error: source path does not exist");
        return;
    }

    if path.is_dir() {
        println!("Source directory is valid");
    } else {
        eprint!("Error: source path is not a directory");
        return;
    }
}
