use zip::result::ZipError;
use std::convert;
use std::io;
use std::io::Read;
use std::fmt;
use curl::easy::Easy;
use std::io::{Error, ErrorKind};
use std::io::BufReader;
use bytesize::ByteSize;
use colored::*;

use super::utils;

use zip::ZipArchive;

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
            PartialZipError::ZipRsError(err) => fmt.write_str(&*err.to_string()),
            PartialZipError::GenericError(s) => fmt.write_str(s),
        }
    }
}

#[derive(Debug)]
pub struct PartialZip {
	url: String,
	archive: ZipArchive<BufReader<PartialReader>>,
}

impl PartialZip {

	pub fn new(url: &str) -> Result<PartialZip, PartialZipError> {
		let reader = PartialReader::new(url)?;
		let bufreader = BufReader::new(reader);
		let archive = ZipArchive::new(bufreader)?;
		// println!("ZipArchive {:?}", archive);
		//TODO
		Ok(PartialZip{
			url:url.to_string(),
			archive,
		})
	}

	pub fn list(&mut self) -> Vec<String> {
		let mut retval = Vec::new();
		for i in 0..self.archive.len() {
			let file = self.archive.by_index(i).unwrap();
			let name = file.name();
			let compressed_size = ByteSize(file.compressed_size());
			let (compression_method, support) = match file.compression() {
				zip::CompressionMethod::Stored => ("Stored".to_string(), "Supported".green().bold()),
				zip::CompressionMethod::Deflated => ("Deflated".to_string(), "Supported".green().bold()),
				zip::CompressionMethod::Bzip2 => ("Bzip2".to_string(), "Supported".green().bold()),
				zip::CompressionMethod::Unsupported(n) => (n.to_string(), "Unsupported".red().bold()),
			};
			let descr = format!("{} - {} - Compression Method: {} {}", 
				name, compressed_size, compression_method, support);
			retval.push(descr);
		}
		retval
	}

	pub fn download(&mut self, filename: &str) -> Result<Vec<u8>, PartialZipError> {
		let mut file = self.archive.by_name(filename)?;
		let mut retval = Vec::with_capacity(file.compressed_size() as usize);
		file.read_to_end(&mut retval)?;
		Ok(retval)
	}

}

#[derive(Debug)]
pub struct PartialReader {
	url: String,
	file_size: u64,
	easy: Easy,
	pos: u64,
}

impl PartialReader {
	pub fn new(url: &str) -> Result<PartialReader, PartialZipError> {
		if !utils::url_is_valid(url) {
            return Err(PartialZipError::InvalidUrl);
        }

        let mut easy = Easy::new();
        easy.url(url).unwrap();
        easy.follow_location(true).unwrap();
        easy.nobody(true).unwrap();
        easy.write_function(|data| Ok(data.len())).unwrap();
        easy.perform().unwrap();
        let file_size = easy.content_length_download().unwrap() as u64;
		Ok(PartialReader{
			url:url.to_string(),
			file_size,
			easy,
			pos:0,
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
		let end = std::cmp::min(maybe_end, self.file_size-1);
		let range = format!("{}-{}", start, end);
		// println!("range {}", range);

		self.easy.range(&range).unwrap();
        self.easy.get(true).unwrap();

        let mut content: Vec<u8> = Vec::new();
        {
            let mut transfer = self.easy.transfer();
            transfer
                .write_function(|data| {
                    content.extend_from_slice(data);
                    Ok(data.len())
                })
                .unwrap();

            transfer.perform().unwrap();
        };

        let n = io::Read::read(&mut content[..].as_ref(), buf)?;
        self.pos += n as u64;

		Ok(n)
	}
}

impl io::Seek for PartialReader {
	fn seek(&mut self, style: io::SeekFrom) -> io::Result<u64> {

		let (base_pos, offset) = match style {
            io::SeekFrom::Start(n) => { self.pos = n; return Ok(n); }
            io::SeekFrom::End(n) => (self.file_size, n),
            io::SeekFrom::Current(n) => (self.pos, n),
        };

        let new_pos = if offset >= 0 {
            base_pos.checked_add(offset as u64)
        } else {
            base_pos.checked_sub((offset.wrapping_neg()) as u64)
        };
        match new_pos {
            Some(n) => {self.pos = n; Ok(self.pos)}
            None => Err(Error::new(ErrorKind::InvalidInput,
                           "invalid seek to a negative or overflowing position"))
        }
	}

	// fn stream_len(&mut self) -> io::Result<u64> {
 //        Ok(self.file_size)
 //    }

 //    fn stream_position(&mut self) -> io::Result<u64> {
 //        Ok(self.pos)
 //    }
}
