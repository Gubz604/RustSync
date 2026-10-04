use std::println;
use std::path::Path;
use reqwest::blocking::Body;
use std::fs::File;

use crate::scanner::{FileEntry, FileChange, FileState};


pub fn check_server(server_address: &str) -> Result<(), String> {
    let health_url: String = format!("{}/health", server_address);
    let result = reqwest::blocking::get(health_url);

    match result {
        Ok(response) => {
            let status = response.status();
            let response_success = status.is_success();
            let text = response.text();
            match text {
                Ok(body) => {
                    if response_success {
                        Ok(())
                    } else {
                        return Err(format!("Error connecting with server: {}", body));
                    }
                },
                Err(err) => {
                    return Err(format!("Error connecting with server: {}", err));
                }
            }
        },
        Err(err) => {
            return Err(format!("Error connecting with server: {}", err));
        }
    }
}

pub fn send_file(file_entry: &FileEntry, source_path: &Path, server_address: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::new();
    let upload_url: String = format!("{}/upload", server_address);

    let file_result = File::open(source_path.join(&file_entry.path));

    let path = file_entry.path.to_string_lossy().to_string();
    let size = file_entry.size.to_string();
    let hash = &file_entry.hash;

    match file_result {
        Ok(file) => {
            let body = Body::sized(file, file_entry.size);
            // println!("Uploading {} with hash {}", path, hash);   // For debugging
            match client
                .post(upload_url)
                .header("X-File-Path", &path)
                .header("X-File-Size", size)
                .header("X-File-Hash", hash)
                .header("Content-Type", "application/octet-stream")
                .body(body)
                .send()
            {
                Ok(response) => {
                    let response_success = response.status().is_success();
                    let status = response.status();
                    let text = response.text();
                    match text {
                        Ok(body) => {
                            if response_success {
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
            Err(format!("Failed to open {}: {err}", path))
        }
    }
}

pub fn upload_changes(changes: &[FileChange], current_files: &[FileEntry], source_root: &Path, server_address: &str) -> Result<(), String> {
    for change in changes {

        match change.state {
            FileState::New | FileState::Modified => {
                let file = current_files.iter().find(|file_entry| (**file_entry).path == change.path);
                match file {
                    Some(entry) => { 
                        send_file(entry, source_root, server_address)?; 
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