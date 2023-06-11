#![feature(buf_read_has_data_left)]
#![feature(file_create_new)]

mod core;
mod file;
pub mod cloud;
pub mod vfs;
pub mod telegram_backend;

use std::{io, path};
use cloud::error::CloudError;

pub trait CloudBackend {
    fn create(input: impl io::Read, output: impl io::Write) -> Self;
    fn load(&self) -> Result<(), CloudError>;
    fn upload_file(&self, file_path: &path::Path) -> Result<(), CloudError>;
    fn download_file(&self, file_path: &path::Path) -> Result<(), CloudError>;
    fn remove_file(&self, file_path: &path::Path) -> Result<(), CloudError>;
    fn check_file(&self, file_name: &str) -> bool;
    fn close(self) -> Result<(), CloudError>;
}
