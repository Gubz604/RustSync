use sha2::{Digest, Sha256};
use std::fmt::Write as FmtWrite;
use std::io::Read;
use std::fs::File;
use std::path::Path;

pub fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    let mut hasher = Sha256::new();

    let mut file = File::open(path)?;

    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();

    let mut hash_string = String::new();

    for byte in result {
        let _ = write!(hash_string, "{:02x}", byte);
    }

    Ok(hash_string)
}