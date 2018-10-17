extern crate url;
extern crate curl;
extern crate podio;
extern crate inflate;
extern crate zip;
extern crate indicatif;
extern crate bytesize;
extern crate colored;

use url::Url;
use curl::easy::Easy;
use std::io::Cursor;
use zip::spec::{HEADER_SIZE, CentralDirectoryEnd};
use zip::result::ZipError;
use std::convert;
use std::fmt;
use podio::{ReadPodExt, LittleEndian};
use std::io;
use std::str;
use std::cmp::min;

use inflate::inflate_bytes;

use indicatif::{ProgressBar, ProgressStyle};

use bytesize::ByteSize;

use colored::*;


#[derive(Debug, Clone)]
pub struct PartialZip {
    url: String,
    file_size: u64,
    files: Vec<FileInZip>,
}

#[derive(Debug)]
pub enum PartialZipError {
    InvalidUrl,
    FileNotFound,
    UnsupportedCompression(u16),
    ZipRsError(ZipError),
    GenericError(String),
}


impl convert::From<ZipError> for PartialZipError {
    fn from(err: ZipError) -> PartialZipError {
        PartialZipError::ZipRsError(err)
    }
}

impl convert::From<io::Error> for PartialZipError {
    fn from(err: io::Error) -> PartialZipError {
        PartialZipError::ZipRsError(ZipError::Io(err))
    }
}

impl convert::From<String> for PartialZipError {
    fn from(err: String) -> PartialZipError {
        PartialZipError::GenericError(err)
    }
}

impl fmt::Display for PartialZipError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            PartialZipError::InvalidUrl => fmt.write_str("Invalid URL"),
            PartialZipError::FileNotFound => fmt.write_str("File Not Found"),
            PartialZipError::UnsupportedCompression(c) => {
                write!(fmt, "{} is a Unsupported Compression", c)
            }
            PartialZipError::ZipRsError(err) => fmt.write_str(&*err.detail()),
            PartialZipError::GenericError(s) => fmt.write_str(s),
        }
    }
}

#[derive(Debug, Clone)]
struct CDFile {
    pub signature: u32,
    pub version: u16,
    pub version_extract: u16,
    pub flags: u16,
    pub method: u16,
    pub mod_time: u16,
    pub mod_date: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub size: u32,
    pub len_filename: u16,
    pub len_extra: u16,
    pub len_comment: u16,
    pub disk_start: u16,
    pub internal_attr: u16,
    pub external_attr: u32,
    pub offset: u32,
}

impl CDFile {
    pub fn new() -> CDFile {
        CDFile {
            signature: 0,
            version: 0,
            version_extract: 0,
            flags: 0,
            method: 0,
            mod_time: 0,
            mod_date: 0,
            crc32: 0,
            compressed_size: 0,
            size: 0,
            len_filename: 0,
            len_extra: 0,
            len_comment: 0,
            disk_start: 0,
            internal_attr: 0,
            external_attr: 0,
            offset: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct FileInZip {
    pub cdfile: CDFile,
    pub file_name: Option<String>,
}

impl FileInZip {
    pub fn new() -> FileInZip {
        FileInZip {
            cdfile: CDFile::new(),
            file_name: None,
        }
    }
}

#[derive(Debug, Clone)]
struct LocalFile {
    pub signature: u32,
    pub version_extract: u16,
    pub flags: u16,
    pub method: u16,
    pub mod_time: u16,
    pub mod_date: u16,
    pub crc32: u32,
    pub compressed_size: u32,
    pub size: u32,
    pub len_filename: u16,
    pub len_extra: u16,
}

impl LocalFile {
    pub fn new() -> LocalFile {
        LocalFile {
            signature: 0,
            version_extract: 0,
            flags: 0,
            method: 0,
            mod_time: 0,
            mod_date: 0,
            crc32: 0,
            compressed_size: 0,
            size: 0,
            len_filename: 0,
            len_extra: 0,
        }
    }
}


impl PartialZip {
    pub fn new(url: &str) -> Result<PartialZip, PartialZipError> {
        if !url_is_valid(url) {
            return Err(PartialZipError::InvalidUrl);
        }

        let mut easy = Easy::new();
        easy.url(url).unwrap();
        easy.follow_location(true).unwrap();
        easy.nobody(true).unwrap();
        easy.write_function(|data| Ok(data.len())).unwrap();
        easy.perform().unwrap();
        let file_size = easy.content_length_download().unwrap() as u64;

        //get central directory end
        let start: u64 = if file_size > (0xffff + HEADER_SIZE) {
            file_size - 0xffff - HEADER_SIZE
        } else {
            0
        };
        let end: u64 = file_size - 1;
        let range = format!("{}-{}", start, end);

        easy.range(&range).unwrap();
        easy.get(true).unwrap();

        let mut cde: Vec<u8> = Vec::new();
        {
            let mut transfer = easy.transfer();
            transfer
                .write_function(|data| {
                    cde.extend_from_slice(data);
                    Ok(data.len())
                })
                .unwrap();

            transfer.perform().unwrap();
        };

        let mut cde_cursor = Cursor::new(cde);

        let (cde, _) = CentralDirectoryEnd::find_and_parse(&mut cde_cursor)?;
        // println!("{:?}", cde);

        // get central directory
        let start = cde.central_directory_offset;
        let end = cde.central_directory_offset + cde.central_directory_size - 1;

        let range = format!("{}-{}", start, end);

        easy.range(&range).unwrap();
        easy.get(true).unwrap();

        let mut cd: Vec<u8> = Vec::new();
        {
            let mut transfer = easy.transfer();
            transfer
                .write_function(|data| {
                    cd.extend_from_slice(data);
                    Ok(data.len())
                })
                .unwrap();

            transfer.perform().unwrap();
        };

        let mut cd_cursor = Cursor::new(cd);

        let mut files: Vec<FileInZip> = Vec::new();
        for _i in 0..cde.number_of_files {
            let mut filein = FileInZip::new();
            let mut cdf = CDFile::new();
            cdf.signature = cd_cursor.read_u32::<LittleEndian>()?;
            cdf.version = cd_cursor.read_u16::<LittleEndian>()?;
            cdf.version_extract = cd_cursor.read_u16::<LittleEndian>()?;
            cdf.flags = cd_cursor.read_u16::<LittleEndian>()?;
            cdf.method = cd_cursor.read_u16::<LittleEndian>()?;
            cdf.mod_time = cd_cursor.read_u16::<LittleEndian>()?;
            cdf.mod_date = cd_cursor.read_u16::<LittleEndian>()?;
            cdf.crc32 = cd_cursor.read_u32::<LittleEndian>()?;
            cdf.compressed_size = cd_cursor.read_u32::<LittleEndian>()?;
            cdf.size = cd_cursor.read_u32::<LittleEndian>()?;
            cdf.len_filename = cd_cursor.read_u16::<LittleEndian>()?;
            cdf.len_extra = cd_cursor.read_u16::<LittleEndian>()?;
            cdf.len_comment = cd_cursor.read_u16::<LittleEndian>()?;
            cdf.disk_start = cd_cursor.read_u16::<LittleEndian>()?;
            cdf.internal_attr = cd_cursor.read_u16::<LittleEndian>()?;
            cdf.external_attr = cd_cursor.read_u32::<LittleEndian>()?;
            cdf.offset = cd_cursor.read_u32::<LittleEndian>()?;
            let filename = ReadPodExt::read_exact(&mut cd_cursor, cdf.len_filename as usize)?;
            match str::from_utf8(&filename) {
                Ok(v) => filein.file_name = Some(String::from(v)),
                Err(e) => println!("invalid filename {:?}! {:?}", e, cdf),
            };
            let _ =
                ReadPodExt::read_exact(&mut cd_cursor, (cdf.len_comment + cdf.len_extra) as usize)
                    .unwrap();
            filein.cdfile = cdf;
            files.push(filein);
        }

        Ok(PartialZip {
            url: url.to_string(),
            file_size: file_size,
            files: files,
        })
    }

    fn compression_is_supported(&self, method: u16) -> bool {
        return method == 8 || method == 0
    }

    pub fn list(&self) -> Vec<String> {
        self.files
            .iter()
            .filter_map(|f|
                {
                    let name = f.file_name.clone();
                    if name.is_none() {
                        None
                    } else {
                        let supported = if self.compression_is_supported(f.cdfile.method) {
                            "Supported".green().bold()
                        } else {
                            "Unsupported".red().bold()
                        };
                        Some(format!("{} - {} - Compression Method: {} {}", name.unwrap(),
                            ByteSize(f.cdfile.size as u64), f.cdfile.method, supported))
                    }
                }
            )
            .collect()
    }

    pub fn download(&self, filename: &str) -> Result<Vec<u8>, PartialZipError> {
        let f = self.get_file(filename)?;

        // for now we support only deflate...
        if !self.compression_is_supported(f.cdfile.method) {
            return Err(PartialZipError::UnsupportedCompression(f.cdfile.method));
        }

        let have_to_decompress = f.cdfile.method != 0;

        // Download
        let mut easy = Easy::new();
        easy.url(&self.url).unwrap();
        easy.follow_location(true).unwrap();
        easy.nobody(true).unwrap();
        let start = f.cdfile.offset;
        let end = f.cdfile.offset + 30 - 1;
        let range = format!("{}-{}", start, end);
        easy.range(&range).unwrap();
        easy.get(true).unwrap();

        let mut v: Vec<u8> = Vec::new();
        {
            let mut transfer = easy.transfer();
            transfer
                .write_function(|data| {
                    v.extend_from_slice(data);
                    Ok(data.len())
                })
                .unwrap();

            transfer.perform().unwrap();
        }
        let mut cursor_lf = Cursor::new(v);

        let mut lf = LocalFile::new();
        lf.signature = cursor_lf.read_u32::<LittleEndian>()?;
        lf.version_extract = cursor_lf.read_u16::<LittleEndian>()?;
        lf.flags = cursor_lf.read_u16::<LittleEndian>()?;
        lf.method = cursor_lf.read_u16::<LittleEndian>()?;
        lf.mod_time = cursor_lf.read_u16::<LittleEndian>()?;
        lf.mod_date = cursor_lf.read_u16::<LittleEndian>()?;
        lf.crc32 = cursor_lf.read_u32::<LittleEndian>()?;
        lf.compressed_size = cursor_lf.read_u32::<LittleEndian>()?;
        lf.size = cursor_lf.read_u32::<LittleEndian>()?;
        lf.len_filename = cursor_lf.read_u16::<LittleEndian>()?;
        lf.len_extra = cursor_lf.read_u16::<LittleEndian>()?;

        // println!("{:#?}", lf);

        let start = f.cdfile.offset + 30 + (lf.len_filename as u32) + (lf.len_extra as u32);
        let end = start + f.cdfile.compressed_size - 1;
        let range = format!("{}-{}", start, end);
        easy.range(&range).unwrap();

        // Progress bar
        let mut downloaded: u64 = 0;
        let total_size = (end - start) as u64;

        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] \
                    [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
                )
                .progress_chars("#>-"),
        );

        let mut fcontent: Vec<u8> = Vec::new();
        {
            let mut transfer = easy.transfer();
            transfer
                .write_function(|data| {
                    fcontent.extend_from_slice(data);
                    let chunk_len = data.len() as u64;
                    let new = min(downloaded + chunk_len, total_size);
                    downloaded = new;
                    pb.set_position(new);
                    Ok(data.len())
                })
                .unwrap();

            transfer.perform().unwrap();

            pb.finish_with_message("downloaded");
        }

        if have_to_decompress {
            let decoded = inflate_bytes(&fcontent)?;
            return Ok(decoded);
        } else {
            return Ok(fcontent);
        }


    }

    fn get_file(&self, filename: &str) -> Result<FileInZip, PartialZipError> {
        for f in self.files.iter() {
            if f.file_name.is_some() {
                if f.file_name.clone().unwrap() == filename {
                    //how to avoid those clones?
                    return Ok(f.clone());
                }
            }
        }
        Err(PartialZipError::FileNotFound)
    }
}

fn url_is_valid(url: &str) -> bool {
    if let Ok(url) = Url::parse(url) {
        if url.scheme() == "http" || url.scheme() == "https" || url.scheme() == "ftp" {
            return true;
        }
    }
    return false;
}
