use conv::{NoError, ValueFrom};
use curl::easy::Easy;
use log::warn;
use num_traits::ToPrimitive;
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
    /// The URL is invalid
    #[error("Invalid URL")]
    InvalidUrl,
    /// The file is not found
    #[error("File Not Found")]
    FileNotFound,
    /// Range request not supported
    #[error("Range request not supported")]
    RangeNotSupported,
    /// The compression scheme is currently not supported
    #[error("{0} is a Unsupported Compression")]
    UnsupportedCompression(u16),
    /// Error for the underlying zip crate
    #[error("zip error: {0}")]
    ZipRsError(#[from] ZipError),
    /// `std::io::Error` wrapper
    #[error("io error: {0}")]
    IOError(#[from] io::Error),
    /// Error for CURL
    #[error("CURL error: {0}")]
    CURLError(#[from] curl::Error),
    /// NoError error
    #[error("NoError error: {0}")]
    NoError(#[from] NoError),
    /// Conversion Error
    #[error("Conversion error: {0}")]
    ConvError(#[from] conv::PosOverflow<u64>),
}

/// Core struct of the crate representing a zip file we want to access partially
#[derive(Debug)]
pub struct PartialZip {
    /// URL of the zip archive
    pub url: String,
    /// The archive object
    pub archive: ZipArchive<BufReader<PartialReader>>,
}

/// Struct for a file in the zip file with some attributes
#[derive(Debug, PartialEq, Eq)]
pub struct PartialZipFile {
    /// Filename
    pub name: String,
    /// Compressed size of the file
    pub compressed_size: u64,
    /// How it has been compressed (compression method, like bzip2, deflate, etc.)
    pub compression_method: zip::CompressionMethod,
    /// Is the compression supported or not by this crate?
    pub supported: bool,
}

impl PartialZip {
    /// Create a new [`PartialZip`]
    /// # Errors
    ///
    /// Will return a [`PartialZipError`] enum depending on what error happened
    pub fn new(url: &dyn ToString) -> Result<Self, PartialZipError> {
        Self::new_check_range(url, false)
    }
    /// Create a new [`PartialZip`]
    /// # Errors
    ///
    /// Will return a [`PartialZipError`] enum depending on what error happened
    pub fn new_check_range(url: &dyn ToString, check_range: bool) -> Result<Self, PartialZipError> {
        let reader = PartialReader::new_check_range(url, check_range)?;
        let bufreader = BufReader::new(reader);
        let archive = ZipArchive::new(bufreader)?;
        Ok(Self {
            url: url.to_string(),
            archive,
        })
    }
    /// Get a list of the files in the archive
    pub fn list(&mut self) -> Vec<PartialZipFile> {
        let mut file_list = Vec::new();
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
                    let pzf = PartialZipFile {
                        name: file.name().to_string(),
                        compressed_size: file.compressed_size(),
                        compression_method,
                        supported,
                    };
                    file_list.push(pzf);
                }
                Err(e) => {
                    // We are unable to get a file, let's try to continue,
                    // and at least return the files we can
                    warn!("list: error while matching file by index: {} - {}", i, e);
                    continue;
                }
            };
        }
        file_list
    }
    /// Download a single file from the archive
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] depending on what happened
    pub fn download(&mut self, filename: &str) -> Result<Vec<u8>, PartialZipError> {
        let mut file = self.archive.by_name(filename)?;
        let mut content = Vec::with_capacity(usize::value_from(file.compressed_size())?);
        file.read_to_end(&mut content)?;
        Ok(content)
    }

    /// Download a single file from the archive showing a progress bar
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] depending on what happened
    #[cfg(feature = "progressbar")]
    pub fn download_with_progressbar(
        &mut self,
        filename: &str,
    ) -> Result<Vec<u8>, PartialZipError> {
        use indicatif::ProgressBar;

        let file = self.archive.by_name(filename)?;
        let mut content = Vec::with_capacity(usize::value_from(file.compressed_size())?);
        let pb = ProgressBar::new(file.compressed_size());
        io::copy(&mut pb.wrap_read(file), &mut content)?;
        Ok(content)
    }
}

/// Reader for the partialzip doing only the partial read instead of downloading everything
#[derive(Debug)]
pub struct PartialReader {
    /// URL at which we read the file
    pub url: String,
    file_size: u64,
    easy: Easy,
    pos: u64,
}

const HTTP_PARTIAL_CONTENT: u32 = 206;

impl PartialReader {
    /// Creates a new [`PartialReader`]
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] enum depending on what happened
    pub fn new(url: &dyn ToString) -> Result<Self, PartialZipError> {
        Self::new_check_range(url, false)
    }
    /// Creates a new [`PartialReader`]
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] enum depending on what happened

    pub fn new_check_range(url: &dyn ToString, check_range: bool) -> Result<Self, PartialZipError> {
        let url = &url.to_string();
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

        if check_range {
            // check if range-request is possible by request 1 byte. if 206 Partial Content (HTTP_PARTIAL_CONTENT) is returned, we can make future request.
            easy.range("0-0")?;
            easy.nobody(true)?;
            easy.perform()?;
            let head_size = easy.content_length_download()?.to_u64().ok_or_else(|| {
                std::io::Error::new(ErrorKind::InvalidData, "can not perform range request")
            })?;
            if head_size != 1 {
                return Err(PartialZipError::RangeNotSupported);
            }
            // 206 Partial Content (HTTP_PARTIAL_CONTENT)
            if easy.response_code()? != HTTP_PARTIAL_CONTENT {
                return Err(PartialZipError::RangeNotSupported);
            }
            easy.range("")?;
            easy.nobody(false)?;
        }
        Ok(Self {
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
        // start = current position
        let start = self.pos;
        // end candidate = start + buf.len() - 1;
        let maybe_end = start
            .checked_add(buf.len().to_u64().ok_or_else(|| {
                std::io::Error::new(
                    ErrorKind::InvalidData,
                    format!("The buf len is invalid {}", buf.len()),
                )
            })?)
            .ok_or_else(|| {
                std::io::Error::new(
                    ErrorKind::InvalidData,
                    format!("start + buf.len() overflow {start} {}", buf.len()),
                )
            })?
            .checked_sub(1)
            .ok_or_else(|| {
                std::io::Error::new(
                    ErrorKind::InvalidData,
                    format!("start + buf.len() - 1 underflow {start} {}", buf.len()),
                )
            })?;
        // end = min(end candidate, file_size - 1)
        let end = std::cmp::min(
            maybe_end,
            self.file_size.checked_sub(1).ok_or_else(|| {
                std::io::Error::new(
                    ErrorKind::InvalidData,
                    format!("file_size - 1 underflow {}", self.file_size),
                )
            })?,
        );
        // check if the end and start are valid ( end >= start )
        if end < start {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                format!("end < start: {end} < {start}"),
            ));
        }
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
        // new position = position + read amount;
        self.pos = self
            .pos
            .checked_add(n.to_u64().ok_or_else(|| {
                std::io::Error::new(ErrorKind::InvalidData, format!("invalid read amount {n}"))
            })?)
            .ok_or_else(|| {
                std::io::Error::new(
                    ErrorKind::InvalidData,
                    format!("adding {n} overflows the reader position {}", self.pos),
                )
            })?;

        Ok(n)
    }
}

impl io::Seek for PartialReader {
    fn seek(&mut self, style: io::SeekFrom) -> io::Result<u64> {
        // we can seek both from start, end, or current position
        let (base_pos, offset) = match style {
            io::SeekFrom::Start(n) => {
                self.pos = n;
                return Ok(n);
            }
            io::SeekFrom::End(n) => (self.file_size, n),
            io::SeekFrom::Current(n) => (self.pos, n),
        };

        let new_pos = if offset >= 0 {
            // position = base position + offset
            base_pos.checked_add(
                u64::value_from(offset)
                    .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e.to_string()))?,
            )
        } else {
            // position = base position - offset
            base_pos.checked_sub(
                u64::value_from(offset.wrapping_neg())
                    .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e.to_string()))?,
            )
        };
        // check if new position is valid
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
