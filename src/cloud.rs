pub mod error;

use std::{fs::{self, File}, io::{self, ErrorKind, Read, Write}, thread, path::{Path, PathBuf}, time::Duration, cell::RefCell};
use crate::file::{Options as SeparationOptions,file_separation::{EncodeErrors, SeparationFile}, *};

use self::error::CloudError;
use crate::CloudBackend;
use crate::vfs::*;
use crate::vfs::error::VFSError;


#[derive(Debug, Clone)]
struct CloudOptions {
    work_dir: PathBuf
}

#[derive(Debug, Clone)]
pub struct Cloud<T: CloudBackend> {
    fs: RefCell<VirtualFileSystem>,
    backend: T,
    option: CloudOptions,
}

impl<T: CloudBackend> Cloud<T> {
    pub fn new() -> Self {

        let try_open_vfs = File::open("vfs.json");

        let vfs_from_backup =
            match try_open_vfs {
                Ok(mut f) => serde_json::from_reader::<File, VirtualFileSystem>(f).unwrap(),

                Err(e) => match e.kind() {
                    ErrorKind::NotFound => VirtualFileSystem::new(FSOption::default()),
                    _ => panic!("{}", e)
                }
            };

        Cloud {
            fs: RefCell::new(vfs_from_backup),
            backend: T::create(io::stdin(), io::stdout()),
            option: CloudOptions {
                work_dir: PathBuf::from("./td/file/documents/")
            },
        }
    }

    /// Получить файл из виртуальной файловой системы, *CloudError* в обратном случае
    pub fn get_file(&self, path: &Path) -> Result<VFSFile, CloudError> {
        self.fs
            .borrow()
            .get_file(path)
            .map(|file| file.clone())
            .map_err(|err| err.into())
    }

    /// Получить папку из виртуальной файловой системы, *CloudError* в обратном случае
    pub fn get_folder(&self, path: &Path) -> Result<VFSFolder, CloudError> {
        self.fs
            .borrow()
            .get_folder(path)
            .map(|folder| folder.clone())
            .map_err(|err| err.into())
    }

    /// Добавляет файл в вирутальную файловую систему
    fn add_file(&self, separation_file: &SeparationFile, virtual_path: &Path) -> Result<(), VFSError> {
        let parts_name = separation_file.parts
            .iter()
            .map(|part| part.part_file_name.clone())
            .collect::<Vec<String>>();

        let metafile_name = separation_file.metafile.clone();

        let v_file = VFSFile {
            name: separation_file.filename.clone(),
            extension: separation_file.file_extension.clone(),
            build_metafile: metafile_name,
            parts_name,
            metadata: Default::default(),
        };

        let res = self.fs.borrow_mut().add_file(virtual_path, v_file);

        self.fs.borrow().save_vfs().unwrap();

        return res;
    }

    /// Добавляет папку в вирутальную файловую систему
    fn add_folder(&self, virtual_path: &Path) -> Result<(), VFSError> {

        let mut virtual_path = PathBuf::from(virtual_path);
        let folder_name = virtual_path.file_name().unwrap().to_string_lossy().to_string();
        virtual_path.pop();

        self.fs.borrow_mut().add_folder(&virtual_path, VFSFolder {
            name: folder_name,
            metadata: Default::default(),
            children: Default::default(),
        })
    }

    /// Удаляет файл из вирутальной файловой системы
    pub fn remove_file(&self, path_file: &Path) -> Result<(), CloudError> {
        let res = self.fs
            .borrow_mut()
            .remove_node(path_file)
            .map_err(|e| e.into());

        self.fs.borrow().save_vfs().unwrap();

        return res;
    }

    /// Удаляет папку из вирутальной файловой системы
    pub fn remove_folder(&self, path_file: &Path) -> Result<(), CloudError> {
        let res = self.fs
            .borrow_mut()
            .remove_node(path_file)
            .map_err(|e| e.into());

        self.fs.borrow().save_vfs().unwrap();

        return res;
    }

    /// Загружает файл в облако
    pub async fn async_upload_file(&self, file_path: &PathBuf, virtual_path: &Path) -> Result<(), CloudError> {

        let options = SeparationOptions {
            path_for_save: Some(self.option.work_dir.clone()),
            count_parts: None,
            part_size: None,
            compressed: None,
        };

        let separation_file =
            dbg!(file_separation::encode_file(dbg!(file_path), options)?);

        self.add_file(&separation_file, virtual_path)?;

        for part_file in &separation_file.parts {

            let mut part_path = self.option.work_dir.clone();
            part_path.push(&part_file.part_file_name);

            self.backend.upload_file(&part_path)?;
        }

        let mut metafile_path = self.option.work_dir.clone();
        metafile_path.push(&separation_file.metafile);

        self.backend.upload_file(&metafile_path)?;

        self.fs.borrow().save_vfs().unwrap();
        return Ok(());
    }

    /// Скачивает файл из облака
    pub async fn async_download_file(&self, virtual_path: &Path) -> Result<PathBuf, CloudError> {

        let v_fs = self.fs.borrow();
        let v_file = v_fs.get_file(virtual_path)?;

        for part in &v_file.parts_name {

            let part_path = format!("{}{}", self.option.work_dir.display(), part);
            let _ = self.backend.download_file(Path::new(&part_path)).unwrap();
        }
        let _ = self.backend.download_file(Path::new(&v_file.build_metafile)).unwrap();

        let metafile_path = format!("{}{}", self.option.work_dir.display(), v_file.build_metafile);

        //let metafile_path = format!("{}{}", self.option.work_dir.display(), v_file.build_metafile);
        let output_file = file_assembly::decode_file(
            &PathBuf::from(&metafile_path),
            PathBuf::from(&self.option.work_dir)
        ).unwrap();

        self.fs.borrow().save_vfs().unwrap();

        Ok(PathBuf::from(format!(
            "{}{}.{}",
            self.option.work_dir.display(),
            v_file.name,
            v_file.extension
        )))
    }
}
