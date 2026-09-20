use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, PartialEq)]
pub enum FileState {
    New,
    Modified,
    Unchanged,
    Deleted,
}

pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub hash: String,
}

impl FileEntry {
    pub fn new(path: PathBuf, size: u64, modified: SystemTime, hash: String) -> Self {
        Self {
            path,
            size,
            modified,
            hash,
        }
    }

    
    pub fn compare(&self, file: &FileEntry) -> FileState {
        if self.path == file.path && self.size == file.size && self.hash == file.hash {
            return FileState::Unchanged
        }

        FileState::Modified
    }
}