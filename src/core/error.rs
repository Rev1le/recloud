use std::io;

#[derive(Debug)]
pub enum TDAppError {
    IOError(io::Error),
}

impl From<io::Error> for TDAppError {
    fn from(value: io::Error) -> Self {
        TDAppError::IOError(value)
    }
}