use std::fmt;

#[derive(Debug)]
pub enum VFSError {
    NodeNotFound,
    FolderNotFound,
    FileNotFound,
    NodeNotRemove(Box<dyn std::error::Error + 'static>),
    FileAlreadyExists,
    FolderAlreadyExists,
    PathError {
        message: String,
    },
}


impl fmt::Display for VFSError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for VFSError { }