use std::fs;
use std::path::Path;

use crate::scanner::{FileChange, FileEntry, FileState};
use crate::hashing::hash_file;

pub fn backup_files(
    changes: &[FileChange],
    current_files: &[FileEntry],
    source_root: &Path,
    backup_root: &Path,
) -> Result<(), std::io::Error> {
    for change in changes {
        let should_backup: bool = match change.state {
            FileState::Modified | FileState::New => true,
            FileState::Deleted | FileState::Unchanged => false,
        };

        if !should_backup {
            continue;
        }

        let source_file = source_root.join(&change.path);
        let backup_file = backup_root.join(&change.path);

        if let Some(path) = backup_file.parent() {
            fs::create_dir_all(path)?;
            let bytes = fs::copy(source_file, &backup_file)?;
            println!("Copied {} ({} bytes)", change.path.display(), bytes);
        }

        let invalid_data_error = std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Could not find current file entry for {}",
                change.path.display()
            )
        );

        let hash_mismatch_error = std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Backup verification failed for {}",
                change.path.display()
            )
        );

        let hash_backup = hash_file(&backup_file)?;
        let Some(current_file) = current_files
            .iter()
            .find(|file| file.path == change.path)
        else {
            return Err(invalid_data_error);
        };

        if current_file.hash != hash_backup {
            return Err(hash_mismatch_error);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::SystemTime;
    use std::path::PathBuf;
    use crate::scanner::FileState;

    #[test]
    fn backup_rejects_hash_mismatch() {

        let file_entry_vector: Vec<FileEntry> = vec![
            FileEntry::new(PathBuf::from("test.txt"), 5, SystemTime::now(), String::from("this_is_a_hash"))
        ];
        let file_change_vector: Vec<FileChange> = vec![
            FileChange { path: file_entry_vector[0].path.to_path_buf() , state: FileState::Modified }
        ];

        let test_dir = std::env::temp_dir().join("rustsync_backup_hash_test");
        let source = test_dir.join("source/test");
        let backup = test_dir.join("backup/test");

        if test_dir.exists() {
            fs::remove_dir_all(&test_dir).unwrap();
        }

        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("test.txt"), b"hello").unwrap();

        let result = backup_files(
            &file_change_vector,
            &file_entry_vector,
            &source,
            &backup,
        );

        assert!(result.is_err());

        let error = result.unwrap_err();

        assert_eq!(
            error.kind(),
            std::io::ErrorKind::InvalidData
        );

        fs::remove_dir_all(&test_dir).unwrap();
    }
}