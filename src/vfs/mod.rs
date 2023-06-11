pub mod error;

use std::{collections::HashMap, ffi::OsStr, fmt, fs, io, path::{Path, PathBuf}};

use serde::{Serialize, Deserialize};

use error::VFSError;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct FSOption {
    version: i64,
    owner: String
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Metadata;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileSystemNode {
    File(VFSFile),
    Folder(VFSFolder)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VFSFile {
    pub name: String,
    pub extension: String,
    pub build_metafile: String,
    pub parts_name: Vec<String>,
    pub metadata : Metadata
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VFSFolder {
    pub name: String,
    pub metadata : Metadata,
    pub children: HashMap<String, FileSystemNode>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualFileSystem {
    pub dirs: HashMap<String, FileSystemNode>,
    pub options: FSOption,
}

impl VirtualFileSystem {

    pub fn new(options: FSOption) -> Self {
        Self {
            dirs: HashMap::from([
                (
                    "fs:".to_string(),
                    FileSystemNode::Folder(VFSFolder {
                        name: "Root".to_string(),
                        metadata: Default::default(),
                        children: HashMap::default(),
                    })
                )
            ]),
            options,
        }
    }

    /// Сохраняет вирутальную файловую систему в файл *vfs.json*
    pub fn save_vfs(&self) -> io::Result<()> {
        let vfs_json = serde_json::to_string(&self).unwrap();
        fs::write("vfs.json", vfs_json.as_bytes())?;

        Ok(())
    }

    /// Возвращает вирутальную файловую систему в json формате как строку
    pub fn get_fs_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    /// Получение файла по вирутальному пути
    pub fn get_file(&self, path: &Path) -> Result<&VFSFile, VFSError> {

        return match self.get_fs_node(path)? {
            FileSystemNode::File(file) => Ok(file),
            FileSystemNode::Folder(_) => return Err(VFSError::FileNotFound)
        }

    }

    /// Получение мутабельного файла по вирутальному пути
    pub fn get_mut_file(&mut self, path: &Path) -> Result<&mut VFSFile, VFSError> {

        return match self.get_mut_fs_node(path)? {
            FileSystemNode::File(file) => Ok(file),
            FileSystemNode::Folder(_) => return Err(VFSError::FileNotFound)
        }

    }

    /// Получение папки по вирутальному пути
    pub fn get_folder(&self, path: &Path) -> Result<&VFSFolder, VFSError> {

        return match self.get_fs_node(path)? {
            FileSystemNode::File(_) => Err(VFSError::FolderNotFound),
            FileSystemNode::Folder(folder) => Ok(folder)
        }

    }

    /// Получение мутабельной папки по вирутальному пути
    pub fn get_mut_folder(&mut self, path: &Path) -> Result<&mut VFSFolder, VFSError> {

        return match self.get_mut_fs_node(path)? {
            FileSystemNode::File(_) => Err(VFSError::FolderNotFound),
            FileSystemNode::Folder(folder) => Ok(folder)
        }

    }

    /// Добавление файла по виртуальному пути
    pub fn add_file(&mut self, path: &Path, file: VFSFile) -> Result<(), VFSError> {

        let folder_for_add = self.get_mut_fs_node(path)?;

        return match folder_for_add {
            FileSystemNode::Folder(folder) => {

                if folder.children.contains_key(&file.name) {
                    return Err(VFSError::FileAlreadyExists);
                }

                folder.children.insert(
                    file.name.clone(),
                    FileSystemNode::File(file)
                );

                Ok(())
            }
            FileSystemNode::File { .. } =>
                Err(VFSError::PathError {
                    message: String::from("Передан путь до файла, а не до директории.")
                })
        }
    }

    /// Добавление папки по виртуальному пути
    pub fn add_folder(&mut self, path: &Path, folder: VFSFolder) -> Result<(), VFSError> {

        let current_folder = self.get_mut_folder(path)?;

        if current_folder.children.contains_key(&folder.name) {
            return Err(VFSError::FolderAlreadyExists);
        }

        current_folder.children.insert(
            folder.name.clone(),
            FileSystemNode::Folder(folder)
        );

        Ok(())
    }

    /// Удаление узла у виртуального пути
    pub fn remove_node(&mut self, path: &Path) -> Result<(), VFSError> {

        let mut path = PathBuf::from(path);

        let remove_name = path
            .iter()
            .last()
            .ok_or(VFSError::PathError {message: String::from("Элемент удаления не найден в пути")})?
            .to_string_lossy()
            .to_string();
        path.pop();

        let folder = self.get_mut_folder(&path)?;

        folder.children.remove(&remove_name).ok_or(
            VFSError::NodeNotRemove(
                Box::new(VFSError::NodeNotFound)
            )
        )?;

        Ok(())
    }

    /// Получение мутабельного узла виртуального пути
    fn get_mut_fs_node(&mut self, path: &Path) -> Result<&mut FileSystemNode, VFSError> {

        let mut path_iter = path.into_iter();

        let root_node_name = path_iter.next().ok_or(
            VFSError::PathError {
                message: String::from("Передан пустой путь!!")
            }
        )?;

        if root_node_name != OsStr::new("fs:") {
            return Err(VFSError::PathError {
                message: String::from("Корень пути не соответсвует fs://")
            })
        }

        let mut current_node = self.dirs.get_mut("fs:").unwrap();

        for path_part in path_iter {

            let path_part = &*path_part.to_string_lossy();

            match current_node {

                &mut FileSystemNode::Folder (ref mut folder) => {

                    current_node = folder.children
                        .get_mut(path_part)
                        .ok_or(VFSError::PathError {
                            message: String::from("VFS не содержи узла пути")
                        })?;
                },

                _ => return Err(VFSError::PathError {
                    message: String::from("Узел пути представляет файл, ожидалась папка")
                })
            }
        }

        return Ok(current_node);
    }

    /// Получение узла виртуального пути
    fn get_fs_node(&self, path: &Path) -> Result<&FileSystemNode, VFSError> {
        let mut path_iter = path.into_iter();

        let root_node_name = path_iter.next().ok_or(
            VFSError::PathError {
                message: String::from("Передан пустой путь!!")
            }
        )?;

        if root_node_name != OsStr::new("fs:") {
            return Err(VFSError::PathError {
                message: String::from("Корень пути не соответсвует fs://")
            })
        }

        let mut current_node = self.dirs.get("fs:").unwrap();

        for path_part in path_iter {

            let path_part = &*path_part.to_string_lossy();

            match current_node {

                &FileSystemNode::Folder (ref folder) => {

                    current_node = folder.children
                        .get(path_part)
                        .ok_or(VFSError::PathError {
                            message: String::from("VFS не содержи узла пути")
                        })?;
                },

                _ => return Err(VFSError::PathError {
                    message: String::from("Узел пути представляет файл, ожидалась папка")
                })
            }
        }

        return Ok(current_node);
    }
}

impl fmt::Display for VirtualFileSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(&self).unwrap())
    }
}