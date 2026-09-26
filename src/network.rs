use std::{eprintln, fs, println};
use serde::{Serialize, de::Unexpected::Bytes};
use std::path::Path;

use crate::scanner::FileEntry;

#[derive(Serialize)]
struct BackupRequest {
    path: String,
    size: u64,
    hash: String,
}


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

pub fn  send_backup_data(file_entry: &FileEntry) -> bool {
    let client = reqwest::blocking::Client::new();


    let path = file_entry.path.to_string_lossy().to_string();
    let size = file_entry.size;
    let hash = file_entry.hash.clone();

    let result = client
        .post("http://localhost:8080/backup")
        .json(&BackupRequest { path, size, hash })
        .send();

    match result {
        Ok(response) => {
            let response_success = response.status().is_success();
            let text = response.text();
            match text {
                Ok(body) => {
                    if body == "OK" && response_success {
                        println!("Backup data sent successful");
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

pub fn send_file(file_path: &Path) -> bool {
    let client = reqwest::blocking::Client::new();

    let bytes = fs::read(file_path);

    match bytes {
        Ok(bytes) => {
            match client
                .post("http://localhost:8080/upload")
                .body(bytes)
                .send()
            {
                Ok(response) => {
                    let response_success = response.status().is_success();
                    let text = response.text();
                    match text {
                        Ok(body) => {
                            if body == "OK" && response_success {
                                println!("File bytes successfully sent");
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
                    eprintln!("Failed to send bytes: {err}");
                    false
                }
            }
        },
        Err(err) => {
            eprintln!("Failed to send bytes: {err}");
            false
        }
    }
}