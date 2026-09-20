use std::fs;
use std::path::Path;

use crate::scanner::{FileChange, FileState};

pub fn backup_files(
    changes: &[FileChange],
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
            let bytes = fs::copy(source_file, backup_file)?;
            println!("Copied {} ({} bytes)", change.path.display(), bytes);
        }
    }

    Ok(())
}
