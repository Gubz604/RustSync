use std::env;

fn main() {
    println!("RustSync");

    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        println!("Source directory: {}", args[1]);
    } else {
        println!("Usage: rustsync <source_directory>");
    }
}
