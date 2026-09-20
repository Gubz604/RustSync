use std::{eprintln, println};



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