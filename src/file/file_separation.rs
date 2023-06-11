use std::{
    fs::{self, File},
    io::{self, BufRead, Write, BufReader, Error},
    path::PathBuf,
    process::ExitCode
};
use std::path::Path;
use uuid::Uuid;

use super::{CompositeFile, FilePart, Options};

#[derive(Debug)]
pub enum EncodeErrors {
    IOError(::std::io::Error),
    OsStringError(std::ffi::OsString),
    PathParseError,
}

impl From<std::io::Error> for EncodeErrors {
    fn from(value: std::io::Error) -> Self {
        EncodeErrors::IOError(value)
    }
}

impl From<std::ffi::OsString> for EncodeErrors {
    fn from(value: std::ffi::OsString) -> Self {
        EncodeErrors::OsStringError(value)
    }
}

#[derive(Debug, Clone)]
pub struct SeparationFile {
    pub filename: String,
    pub file_extension: String,
    pub metafile: String,
    pub parts: Vec<FilePart>,
    pub options: Options
}

pub fn encode_file(path: &PathBuf, options: Options) -> Result<SeparationFile, EncodeErrors> {

    if !path.is_file() {
        println!("{}", path.display());
        panic!("Предоставьте путь к файлу")
    }

    let path_for_save = dbg!(options.clone().path_for_save.unwrap_or(PathBuf::new()));

    println!("{}", path_for_save.is_dir());

    if !path_for_save.is_dir() {
        return dbg!(Err(EncodeErrors::PathParseError));
    }
    let file = File::open(&path)?;

    let mut size_part = options.part_size.unwrap_or(1_073_741_824_usize);

    let mut f  = BufReader::with_capacity(
        size_part,
        file
    );

    let mut composite_file = CompositeFile {
        filename: path
            .file_stem()
            .ok_or(EncodeErrors::PathParseError)?
            .to_os_string()
            .into_string()?,
        file_extension:  path
            .extension()
            .ok_or(EncodeErrors::PathParseError)?
            .to_os_string()
            .into_string()?,
        file_len: f.capacity(),
        parts: vec![],
        uuid_parts: Uuid::new_v4().to_string(),
    };

    let max_count_parts = options.count_parts.unwrap_or(255);

    let mut number_part = 1;

    let mut parts = vec![];

    while f.has_data_left()? {

        if number_part > max_count_parts {
            println!(
                "Файл слишком большой для размещения в {} частей",
                max_count_parts
            );
            std::process::exit(1);
        }

        let buffer_bytes = f.fill_buf()?;
        let buffer_bytes_len = buffer_bytes.len();

        let part = encode_part(
            &composite_file.uuid_parts,
            number_part,
            buffer_bytes,
            &path_for_save
        )?;

        parts.push(part.clone());

        composite_file.parts.push(part);

        number_part += 1;
        f.consume(buffer_bytes_len);
    }

    let metafile = encode_metafile(&composite_file, &path_for_save)?;

    Ok(SeparationFile {
        filename: composite_file.filename,
        file_extension: composite_file.file_extension,
        metafile,
        parts,
        options,
    })
}

fn encode_part(part_uuid: &str, part_number: u8, data: &[u8], path_for_save: &PathBuf) -> io::Result<FilePart> {

    let part_file_name = format!("{}_{}.part", part_uuid, part_number);
    let hash_bytes = md5::compute(&part_file_name).0.to_vec();

    let mut part_file = File::create_new(format!("{}{}", path_for_save.display(), &part_file_name))?;
    part_file.write_all(&hash_bytes.as_slice())?;
    part_file.write_all(&data)?;
    part_file.flush()?;

    println!("Файл с частью данными был создан => {}{}", path_for_save.display(), &part_file_name);

    Ok(FilePart {
        hash_bytes,
        part_file_name,
    })
}

fn encode_metafile(composite_file: &CompositeFile, path_for_save: &PathBuf) -> io::Result<String> {

    let uuid = Uuid::new_v4().to_string();
    let metafile_name = format!("{}build_file_{}.meta", uuid, composite_file.filename);

    let mut metafile = File::create(
        format!("{}{}", path_for_save.display(), &metafile_name)
    )?;
    let source_filename_bytes = composite_file.filename.as_bytes();
    let source_format_bytes =  composite_file.file_extension.as_bytes();
    let parts_uuid_bytes = composite_file.uuid_parts.as_bytes();

    // Запись имени исходного файла
    metafile.write(
        &(source_filename_bytes.len() as u8).to_be_bytes()
    )?;
    metafile.write(
        source_filename_bytes
    )?;

    // Запись расширения исходного файла
    metafile.write(
        &(source_format_bytes.len() as u8).to_be_bytes()
    )?;
    metafile.write(
        source_format_bytes
    )?;

    // Запись uuid в названии частей.
    metafile.write(
        &(parts_uuid_bytes.len() as u8).to_be_bytes()
    )?;
    metafile.write(
        parts_uuid_bytes
    )?;

    // Запись всех хешей частей как массив
    metafile.write(&composite_file.parts.len().to_be_bytes())?;
    composite_file.parts
        .iter()
        .map(
            |x| metafile.write(&x.hash_bytes).unwrap()
        )
        .for_each(drop);

    Ok(metafile_name)
}