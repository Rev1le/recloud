pub mod file_separation;
pub mod file_assembly;

/// Часть файла представленная массивом байтов
#[derive(Debug, Clone)]
pub struct FilePart {
    pub hash_bytes: Vec<u8>,
    pub part_file_name: String,
}

/// Собираемый файл
#[derive(Debug, Clone)]
pub struct CompositeFile {
    pub filename: String,
    pub file_extension: String,
    pub file_len: usize,
    pub parts: Vec<FilePart>,
    pub uuid_parts: String,
}

/// Опции для настройки *file_separation* и *file_assembly*
#[derive(Debug, Clone)]
pub struct Options {
    pub path_for_save: Option<std::path::PathBuf>,
    pub count_parts: Option<u8>,
    pub part_size: Option<usize>,
    pub compressed: Option<bool>,
}