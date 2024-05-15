use conv::{NoError, ValueFrom};
use curl::easy::Easy;
use num_traits::ToPrimitive;
use std::cell::RefCell;
use std::io;
use std::io::BufReader;
use std::io::ErrorKind;
use std::time::Duration;
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
    /// `NoError` error
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
    url: String,
    /// The archive object
    archive: RefCell<ZipArchive<BufReader<PartialReader>>>,
}

/// Struct for a file in the zip file with some attributes
#[derive(Debug, PartialEq, Eq)]
pub struct PartialZipFileDetailed {
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
        // higher capacity BufReader has better performances
        let bufreader = BufReader::with_capacity(0x0010_0000, reader);
        let archive = ZipArchive::new(bufreader)?;
        Ok(Self {
            url: url.to_string(),
            archive: RefCell::new(archive),
        })
    }

    /// Returns the url for the [`PartialZip`]
    pub fn url(&self) -> String {
        self.url.clone()
    }

    /// Get a list of the filenames in the archive
    pub fn list_names(&self) -> Vec<String> {
        self.archive
            .borrow()
            .file_names()
            .map(std::borrow::ToOwned::to_owned)
            .collect()
    }

    /// Get a list of the files in the archive with details (slow)
    pub fn list_detailed(&self) -> Vec<PartialZipFileDetailed> {
        let mut file_list = Vec::new();
        let num_files = self.archive.borrow().len();
        for i in 0..num_files {
            match self.archive.borrow_mut().by_index(i) {
                Ok(file) => {
                    let compression_method = file.compression();
                    // we only support some compressions
                    let supported = matches!(
                        compression_method,
                        zip::CompressionMethod::Stored
                            | zip::CompressionMethod::Deflated
                            | zip::CompressionMethod::Bzip2
                            | zip::CompressionMethod::Zstd
                    );
                    let pzf = PartialZipFileDetailed {
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
                    log::warn!("list: error while matching file by index: {i} - {e}");
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
    pub fn download(&self, filename: &str) -> Result<Vec<u8>, PartialZipError> {
        let mut content: Vec<u8> = Vec::new();
        self.download_to_write(filename, &mut content)?;
        Ok(content)
    }

    /// Download a single file from the archive and writes it to a [`std::io::Write`]
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] depending on what happened
    pub fn download_to_write(
        &self,
        filename: &str,
        writer: &mut dyn std::io::Write,
    ) -> Result<(), PartialZipError> {
        let mut archive = self.archive.borrow_mut();
        let mut file = archive.by_name(filename)?;
        io::copy(&mut file, writer)?;
        Ok(())
    }

    /// Download a single file from the archive showing a progress bar
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] depending on what happened
    #[cfg(feature = "progressbar")]
    pub fn download_with_progressbar(&self, filename: &str) -> Result<Vec<u8>, PartialZipError> {
        let mut content: Vec<u8> = Vec::new();
        self.download_to_write_with_progressbar(filename, &mut content)?;
        Ok(content)
    }

    /// Download a single file from the archive showing a progress bar to a [`std::io::Write`]
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] depending on what happened
    #[cfg(feature = "progressbar")]
    pub fn download_to_write_with_progressbar(
        &self,
        filename: &str,
        writer: &mut dyn std::io::Write,
    ) -> Result<(), PartialZipError> {
        use indicatif::ProgressBar;

        let mut archive = self.archive.borrow_mut();
        let file = archive.by_name(filename)?;
        let pb = ProgressBar::new(file.compressed_size());
        io::copy(&mut pb.wrap_read(file), writer)?;
        Ok(())
    }
}

/// Reader for the partialzip doing only the partial read instead of downloading everything
#[derive(Debug)]
pub struct PartialReader {
    /// URL at which we read the file
    url: String,
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
        easy.tcp_keepalive(true)?;
        easy.tcp_keepidle(Duration::from_secs(120))?;
        easy.tcp_keepintvl(Duration::from_secs(60))?;
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

    /// Returns the url for the [`PartialReader`]
    #[must_use]
    pub fn url(&self) -> String {
        self.url.clone()
    }
}

impl io::Read for PartialReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        log::trace!(
            "read self.pos = {:x} self.file_size = {:x}",
            self.pos,
            self.file_size
        );
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
        log::trace!("maybe_end = {maybe_end:x}");
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
        log::trace!("end = {end:x} start = {start:x}");
        // check if the end and start are valid ( end >= start )
        if end < start {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                format!("end < start: {end} < {start}"),
            ));
        }
        let range = format!("{start}-{end}");
        log::trace!("range = {range}");

        self.easy.range(&range)?;
        self.easy.get(true)?;

        let mut content: Vec<u8> = Vec::new();
        {
            let mut transfer = self.easy.transfer();
            transfer.write_function(|data| {
                log::trace!("transfered {:x} bytes", data.len());
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
        log::trace!("new self.pos = {:x}", self.pos);
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
        log::trace!("seek base_pos = {base_pos:x} offset = {offset:x}");
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
                log::trace!("new self.pos = {n:x}");
                Ok(self.pos)
            }
            None => Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "invalid seek to a negative or overflowing position",
            )),
        }
    }
}
