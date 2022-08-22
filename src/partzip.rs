use conv::{NoError, ValueFrom};
use curl::easy::Easy;
use num_traits::ToPrimitive;
use std::convert;
use std::io;
use std::io::BufReader;
use std::io::ErrorKind;
use std::io::Read;
use thiserror::Error;
use zip::result::ZipError;

use super::utils;

use zip::ZipArchive;

/// Enum for errors thrown by the partialzip crate
#[derive(Error, Debug)]
pub enum PartialZipError {
    /// the URL is invalid
    #[error("Invalid URL")]
    InvalidUrl,
    /// The file is not found
    #[error("File Not Found")]
    FileNotFound,
    /// The compression scheme is currently not supported
    #[error("{0} is a Unsupported Compression")]
    UnsupportedCompression(u16),
    /// Error for the underlying zip crate
    #[error("zip error: {0}")]
    ZipRsError(ZipError),
    /// Error for CURL
    #[error("CURL error: {0}")]
    CURLError(curl::Error),
    /// Generic catch all string error
    #[error("{0}")]
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

impl convert::From<curl::Error> for PartialZipError {
    fn from(err: curl::Error) -> PartialZipError {
        PartialZipError::CURLError(err)
    }
}

impl convert::From<String> for PartialZipError {
    fn from(err: String) -> PartialZipError {
        PartialZipError::GenericError(err)
    }
}

impl convert::From<NoError> for PartialZipError {
    fn from(err: NoError) -> PartialZipError {
        PartialZipError::GenericError(err.to_string())
    }
}

impl convert::From<conv::PosOverflow<u64>> for PartialZipError {
    fn from(err: conv::PosOverflow<u64>) -> PartialZipError {
        PartialZipError::GenericError(err.to_string())
    }
}

/// Core struct of the crate representing a zip file we want to access partially
#[derive(Debug)]
pub struct PartialZip {
    /// url at which the zip archive resides
    pub url: String,
    /// The archive itself
    pub archive: ZipArchive<BufReader<PartialReader>>,
}

/// struct for a file in the zip file with some attributes
#[derive(Debug, PartialEq, Eq)]
pub struct PartialZipFile {
    /// filename
    pub name: String,
    /// size of the file
    pub compressed_size: u64,
    /// how it has been compressed
    pub compression_method: zip::CompressionMethod,
    /// is the compression supported or not
    pub supported: bool,
}

impl PartialZip {
    /// Create a new `PartialZip`
    /// # Errors
    ///
    /// Will return a `PartialZipError` enum depending on what error happened
    pub fn new(url: &str) -> Result<PartialZip, PartialZipError> {
        let reader = PartialReader::new(url)?;
        let bufreader = BufReader::new(reader);
        let archive = ZipArchive::new(bufreader)?;
        Ok(PartialZip {
            url: url.to_string(),
            archive,
        })
    }
    /// Get a list of the files in the archive
    pub fn list(&mut self) -> Vec<PartialZipFile> {
        let mut retval = Vec::new();
        for i in 0..self.archive.len() {
            match self.archive.by_index(i) {
                Ok(file) => {
                    let compression_method = file.compression();
                    let supported = matches!(
                        compression_method,
                        zip::CompressionMethod::Stored
                            | zip::CompressionMethod::Deflated
                            | zip::CompressionMethod::Bzip2
                            | zip::CompressionMethod::Zstd
                    );
                    let p_file = PartialZipFile {
                        name: file.name().to_string(),
                        compressed_size: file.compressed_size(),
                        compression_method,
                        supported,
                    };
                    retval.push(p_file);
                }
                Err(_) => {
                    // We are unable to get a file, let's try to continue,
                    // and at least return the files we can
                    continue;
                }
            };
        }
        retval
    }
    /// Download a single file from the archive
    ///
    /// # Errors
    /// Will return a `PartialZipError` depending on what happened
    pub fn download(&mut self, filename: &str) -> Result<Vec<u8>, PartialZipError> {
        let mut file = self.archive.by_name(filename)?;
        let mut retval = Vec::with_capacity(usize::value_from(file.compressed_size())?);
        file.read_to_end(&mut retval)?;
        Ok(retval)
    }
}

/// Reader for the partialzip doing only the partial read instead of downloading everything
#[derive(Debug)]
pub struct PartialReader {
    /// url at which we read
    pub url: String,
    file_size: u64,
    easy: Easy,
    pos: u64,
}

impl PartialReader {
    /// Creates a new `PartialReader`
    ///
    /// # Errors
    /// Will return a `PartialZipError` enum depending on what happened

    pub fn new(url: &str) -> Result<PartialReader, PartialZipError> {
        if !utils::url_is_valid(url) {
            return Err(PartialZipError::InvalidUrl);
        }

        let mut easy = Easy::new();
        easy.url(url)?;
        easy.follow_location(true)?;
        easy.nobody(true)?;
        easy.write_function(|data| Ok(data.len()))?;
        easy.perform()?;
        let file_size = easy
            .content_length_download()?
            .to_u64()
            .ok_or_else(|| std::io::Error::new(ErrorKind::InvalidData, "invalid content length"))?;
        Ok(PartialReader {
            url: url.to_string(),
            file_size,
            easy,
            pos: 0,
        })
    }
}

impl io::Read for PartialReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.file_size {
            return Ok(0);
        }
        let start = self.pos;
        let maybe_end = start + (buf.len() as u64) - 1;
        let end = std::cmp::min(maybe_end, self.file_size - 1);
        let range = format!("{start}-{end}");

        self.easy.range(&range)?;
        self.easy.get(true)?;

        let mut content: Vec<u8> = Vec::new();
        {
            let mut transfer = self.easy.transfer();
            transfer.write_function(|data| {
                content.extend_from_slice(data);
                Ok(data.len())
            })?;

            transfer.perform()?;
        };

        let n = io::Read::read(&mut content[..].as_ref(), buf)?;
        self.pos += n as u64;

        Ok(n)
    }
}

impl io::Seek for PartialReader {
    fn seek(&mut self, style: io::SeekFrom) -> io::Result<u64> {
        let (base_pos, offset) = match style {
            io::SeekFrom::Start(n) => {
                self.pos = n;
                return Ok(n);
            }
            io::SeekFrom::End(n) => (self.file_size, n),
            io::SeekFrom::Current(n) => (self.pos, n),
        };

        let new_pos = if offset >= 0 {
            base_pos.checked_add(
                u64::value_from(offset)
                    .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e.to_string()))?,
            )
        } else {
            base_pos.checked_sub(
                u64::value_from(offset.wrapping_neg())
                    .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e.to_string()))?,
            )
        };
        match new_pos {
            Some(n) => {
                self.pos = n;
                Ok(self.pos)
            }
            None => Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "invalid seek to a negative or overflowing position",
            )),
        }
    }
}
