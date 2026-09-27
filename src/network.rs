use std::{eprintln, fs, println};
use std::path::Path;

use crate::scanner::{FileEntry, FileChange, FileState};


pub fn check_server() -> bool {
    let result = reqwest::blocking::get("http://localhost:8080/health");

    match result {
        Ok(response) => {
            let response_success = response.status().is_success();
            let text = response.text();
            match text {
                Ok(body) => {
                    if body == "OK" && response_success {
                        println!("Connection to server successful");
                        true
                    } else {
                        false
                    }
                },
                Err(err) => {
                    eprintln!("Error with connecting to server: {err}");
                    false
                }
            }
        },
        Err(err) => {
            eprintln!("Failed to check server: {err}");
            false
        }
    }
}

pub fn send_file(file_entry: &FileEntry, source_path: &Path) -> Result<(), String> {
    let client = reqwest::blocking::Client::new();

    let bytes = fs::read(source_path.join(&file_entry.path));

    let path = file_entry.path.to_string_lossy().to_string();
    let size = file_entry.size.to_string();
    let hash = &file_entry.hash;

    match bytes {
        Ok(bytes) => {
            match client
                .post("http://localhost:8080/upload")
                .header("X-File-Path", &path)
                .header("X-File-Size", size)
                .header("X-File-Hash", hash)
                .header("Content-Type", "application/octet-stream")
                .body(bytes)
                .send()
            {
                Ok(response) => {
                    let response_success = response.status().is_success();
                    let status = response.status();
                    let text = response.text();
                    match text {
                        Ok(body) => {
                            if body == "OK" && response_success {
                                println!("File bytes successfully sent for {}", path);
                                Ok(())
                            } else {
                                Err(format!("Server rejected {}: {} - {}", path, status, body.trim()))
                            }
                        },
                        Err(err) => {
                            Err(format!("Failed to get text from {}: {err}", path))
                        }
                    }
                },
                Err(err) => {
                    Err(format!("Failed to send {}: {err}", path))
                }
            }
        },
        Err(err) => {
            Err(format!("Failed to read {}: {err}", path))
        }
    }
}

pub fn upload_changes(changes: &[FileChange], current_files: &[FileEntry], source_root: &Path) -> Result<(), String> {
    for change in changes {

        match change.state {
            FileState::New | FileState::Modified => {
                let file = current_files.iter().find(|file_entry| (**file_entry).path == change.path);
                match file {
                    Some(entry) => { 
                        send_file(entry, source_root)?; 
                    } ,
                    None => {
                        return Err(format!("Could not find file {}", change.path.display()));
                    }
                }
            },
            FileState::Unchanged | FileState::Deleted => {
                continue;
            }
        }
    }
    Ok(())
}