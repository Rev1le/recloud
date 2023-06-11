use std::io;
use crate::file::file_separation::EncodeErrors;
use crate::vfs::error::VFSError;

#[derive(Debug)]
pub enum CloudError {
    IOError(io::Error),
    EncodeError(EncodeErrors),
    VFSError(VFSError),
}

impl From<VFSError> for CloudError {
    fn from(value: VFSError) -> Self {
        Self::VFSError(value)
    }
}

impl From<EncodeErrors> for CloudError {
    fn from(value: EncodeErrors) -> Self {
        Self::EncodeError(value)
    }
}