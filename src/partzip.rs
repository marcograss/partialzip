use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::NaiveTime;
use curl::easy::Easy;
use serde::Deserialize;
use serde::Serialize;
use std::cell::RefCell;
use std::fs::File;
use std::io;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::ErrorKind;
use std::path::Path;
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
}

/// Default maximum number of HTTP redirects to follow
pub const DEFAULT_MAX_REDIRECTS: u32 = 10;

/// Default connection timeout in seconds
pub const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 30;

/// Default TCP keep-alive idle time in seconds
pub const DEFAULT_TCP_KEEPIDLE_SECS: u64 = 120;

/// Default TCP keep-alive interval in seconds
pub const DEFAULT_TCP_KEEPINTVL_SECS: u64 = 60;

/// Options for configuring [`PartialZip`] and [`PartialReader`] behavior
#[derive(Debug, Clone)]
pub struct PartialZipOptions {
    /// Whether to verify that the server supports HTTP Range requests
    pub check_range: bool,
    /// Maximum number of HTTP redirects to follow (prevents redirect loops and SSRF attacks)
    pub max_redirects: u32,
    /// Connection timeout (None = no timeout)
    pub connect_timeout: Option<Duration>,
    /// TCP keep-alive idle time before sending probes
    pub tcp_keepidle: Duration,
    /// TCP keep-alive interval between probes
    pub tcp_keepintvl: Duration,
    /// Basic authentication credentials (username, password)
    pub basic_auth: Option<(String, String)>,
    /// Proxy URL (e.g., `http://proxy:8080`, `socks5://proxy:1080`)
    pub proxy: Option<String>,
    /// Proxy authentication credentials (username, password)
    pub proxy_auth: Option<(String, String)>,
    /// Maximum number of retries for transient network errors (0 = no retries)
    pub max_retries: u32,
    /// Base delay between retries (doubles with each attempt for exponential backoff)
    pub retry_base_delay: Duration,
}

impl Default for PartialZipOptions {
    fn default() -> Self {
        Self {
            check_range: false,
            max_redirects: DEFAULT_MAX_REDIRECTS,
            connect_timeout: Some(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS)),
            tcp_keepidle: Duration::from_secs(DEFAULT_TCP_KEEPIDLE_SECS),
            tcp_keepintvl: Duration::from_secs(DEFAULT_TCP_KEEPINTVL_SECS),
            basic_auth: None,
            proxy: None,
            proxy_auth: None,
            max_retries: 0,
            retry_base_delay: Duration::from_secs(1),
        }
    }
}

impl PartialZipOptions {
    /// Create new options with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to check for range request support
    #[must_use]
    pub const fn check_range(mut self, check: bool) -> Self {
        self.check_range = check;
        self
    }

    /// Set the maximum number of redirects to follow
    #[must_use]
    pub const fn max_redirects(mut self, max: u32) -> Self {
        self.max_redirects = max;
        self
    }

    /// Set the connection timeout (None = no timeout)
    #[must_use]
    pub const fn connect_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the TCP keep-alive idle time before sending probes
    #[must_use]
    pub const fn tcp_keepidle(mut self, duration: Duration) -> Self {
        self.tcp_keepidle = duration;
        self
    }

    /// Set the TCP keep-alive interval between probes
    #[must_use]
    pub const fn tcp_keepintvl(mut self, duration: Duration) -> Self {
        self.tcp_keepintvl = duration;
        self
    }

    /// Set basic authentication credentials
    #[must_use]
    pub fn basic_auth(mut self, username: &str, password: &str) -> Self {
        self.basic_auth = Some((username.to_string(), password.to_string()));
        self
    }

    /// Set proxy URL (e.g., `http://proxy:8080`, `socks5://proxy:1080`)
    #[must_use]
    pub fn proxy(mut self, url: &str) -> Self {
        self.proxy = Some(url.to_string());
        self
    }

    /// Set proxy authentication credentials
    #[must_use]
    pub fn proxy_auth(mut self, username: &str, password: &str) -> Self {
        self.proxy_auth = Some((username.to_string(), password.to_string()));
        self
    }

    /// Set the maximum number of retries for transient network errors (0 = disabled)
    #[must_use]
    pub const fn max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Set the base delay between retries (doubles with each attempt for exponential backoff)
    #[must_use]
    pub const fn retry_base_delay(mut self, delay: Duration) -> Self {
        self.retry_base_delay = delay;
        self
    }
}

/// Core struct of the crate representing a zip file we want to access partially
#[derive(Debug)]
pub struct PartialZip {
    /// URL of the zip archive
    url: String,
    /// The archive object
    archive: RefCell<ZipArchive<BufReader<PartialReader>>>,
    /// The archive size
    file_size: u64,
    /// Configuration options (stored for creating parallel connections)
    options: PartialZipOptions,
}

/// Compression methods for the files inside the archive. Redefined structure to make it serializable.
/// Maps directly to the zip crate `zip::CompressionMethod` enum.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartialZipCompressionMethod {
    /// Stored (no compression)
    Stored,
    /// Deflated compression
    Deflated,
    /// bzip2 compression
    Bzip2,
    /// zstd compression
    Zstd,
    /// unsupported compression
    Unsupported,
}

impl From<zip::CompressionMethod> for PartialZipCompressionMethod {
    fn from(value: zip::CompressionMethod) -> Self {
        match value {
            zip::CompressionMethod::Stored => Self::Stored,
            zip::CompressionMethod::Deflated => Self::Deflated,
            zip::CompressionMethod::Bzip2 => Self::Bzip2,
            zip::CompressionMethod::Zstd => Self::Zstd,
            _ => Self::Unsupported,
        }
    }
}

/// Struct for a file in the zip file with some attributes
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartialZipFileDetailed {
    /// Filename
    pub name: String,
    /// Compressed size of the file
    pub compressed_size: u64,
    /// How it has been compressed (compression method, like bzip2, deflate, etc.)
    pub compression_method: PartialZipCompressionMethod,
    /// Is the compression supported or not by this crate?
    pub supported: bool,
    /// The date the file was last modified
    pub last_modified: Option<NaiveDateTime>,
}

impl PartialZip {
    /// Create a new [`PartialZip`] with default options
    /// # Errors
    ///
    /// Will return a [`PartialZipError`] enum depending on what error happened
    pub fn new(url: &str) -> Result<Self, PartialZipError> {
        Self::new_with_options(url, &PartialZipOptions::default())
    }

    /// Create a new [`PartialZip`] with range checking option
    /// # Errors
    ///
    /// Will return a [`PartialZipError`] enum depending on what error happened
    pub fn new_check_range(url: &str, check_range: bool) -> Result<Self, PartialZipError> {
        Self::new_with_options(url, &PartialZipOptions::default().check_range(check_range))
    }

    /// Create a new [`PartialZip`] with custom options
    /// # Errors
    ///
    /// Will return a [`PartialZipError`] enum depending on what error happened
    pub fn new_with_options(
        url: &str,
        options: &PartialZipOptions,
    ) -> Result<Self, PartialZipError> {
        let reader = PartialReader::new_with_options(url, options)?;
        let file_size = reader.file_size;
        // higher capacity BufReader has better performances
        let bufreader = BufReader::with_capacity(0x0010_0000, reader);
        let archive = ZipArchive::new(bufreader)?;
        Ok(Self {
            url: url.to_owned(),
            archive: RefCell::new(archive),
            file_size,
            options: options.clone(),
        })
    }

    /// Returns the url for the [`PartialZip`]
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the zip size for the entire archive of the [`PartialZip`]
    pub const fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Get a list of the filenames in the archive
    pub fn list_names(&self) -> Vec<String> {
        self.archive
            .borrow()
            .file_names()
            .map(std::borrow::ToOwned::to_owned)
            .collect()
    }

    /// Get a list of the files in the archive with details (much slower than just listing names because it fetches much more data around with more requests)
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
                    let (date, time) = file.last_modified().map_or((None, None), |datetime| {
                        (
                            NaiveDate::from_ymd_opt(
                                datetime.year().into(),
                                datetime.month().into(),
                                datetime.day().into(),
                            ),
                            NaiveTime::from_hms_opt(
                                datetime.hour().into(),
                                datetime.minute().into(),
                                datetime.second().into(),
                            ),
                        )
                    });
                    let last_modified = if let (Some(d), Some(t)) = (date, time) {
                        Some(NaiveDateTime::new(d, t))
                    } else {
                        None
                    };
                    let pzf = PartialZipFileDetailed {
                        name: file.name().to_string(),
                        compressed_size: file.compressed_size(),
                        compression_method: compression_method.into(),
                        supported,
                        last_modified,
                    };
                    file_list.push(pzf);
                }
                Err(e) => {
                    // We are unable to get a file, let's try to continue,
                    // and at least return the files we can
                    log::warn!("list: error while matching file by index: {i} - {e}");
                }
            };
        }
        file_list
    }
    /// Download a single file from the archive into memory.
    ///
    /// **Note:** This loads the entire decompressed file into RAM. For large files,
    /// consider using [`download_to_file`](Self::download_to_file) or
    /// [`download_to_write`](Self::download_to_write) which stream directly to disk
    /// without buffering the entire file in memory.
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] depending on what happened
    pub fn download(&self, filename: &str) -> Result<Vec<u8>, PartialZipError> {
        let size = usize::try_from(self.archive.borrow_mut().by_name(filename)?.size())
            .unwrap_or(usize::MAX);
        let mut content: Vec<u8> = Vec::with_capacity(size);
        self.download_to_write(filename, &mut content)?;
        Ok(content)
    }

    /// Download a single file from the archive directly to a file path.
    ///
    /// This is the recommended method for large files as it streams the decompressed
    /// content directly to disk without loading the entire file into memory.
    ///
    /// Returns the number of bytes written.
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] depending on what happened
    pub fn download_to_file(
        &self,
        filename: &str,
        output_path: &Path,
    ) -> Result<u64, PartialZipError> {
        let file = File::create(output_path)?;
        let mut writer = BufWriter::new(file);
        self.download_to_write(filename, &mut writer)
    }

    /// Download a single file from the archive and write it to any [`std::io::Write`] implementor.
    ///
    /// This method streams the decompressed content directly to the writer without
    /// buffering the entire file in memory. Use this for large files or when you need
    /// custom output handling.
    ///
    /// Returns the number of bytes written.
    ///
    /// # Example
    /// ```no_run
    /// use partialzip::PartialZip;
    /// use std::fs::File;
    ///
    /// let pz = PartialZip::new(&"https://example.com/archive.zip")?;
    /// let mut file = File::create("output.bin")?;
    /// pz.download_to_write("large_file.bin", &mut file)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] depending on what happened
    pub fn download_to_write(
        &self,
        filename: &str,
        writer: &mut dyn std::io::Write,
    ) -> Result<u64, PartialZipError> {
        let mut archive = self.archive.borrow_mut();
        let mut file = archive.by_name(filename)?;
        let bytes_written = io::copy(&mut file, writer)?;
        Ok(bytes_written)
    }

    /// Download a single file from the archive into memory, showing a progress bar.
    ///
    /// **Note:** This loads the entire decompressed file into RAM. For large files,
    /// consider using [`download_to_file_with_progressbar`](Self::download_to_file_with_progressbar)
    /// or [`download_to_write_with_progressbar`](Self::download_to_write_with_progressbar).
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] depending on what happened
    #[cfg(feature = "progressbar")]
    pub fn download_with_progressbar(&self, filename: &str) -> Result<Vec<u8>, PartialZipError> {
        let size = usize::try_from(self.archive.borrow_mut().by_name(filename)?.size())
            .unwrap_or(usize::MAX);
        let mut content: Vec<u8> = Vec::with_capacity(size);
        self.download_to_write_with_progressbar(filename, &mut content)?;
        Ok(content)
    }

    /// Download a single file from the archive directly to a file path, showing a progress bar.
    ///
    /// This is the recommended method for large files as it streams the decompressed
    /// content directly to disk without loading the entire file into memory.
    ///
    /// Returns the number of bytes written.
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] depending on what happened
    #[cfg(feature = "progressbar")]
    pub fn download_to_file_with_progressbar(
        &self,
        filename: &str,
        output_path: &Path,
    ) -> Result<u64, PartialZipError> {
        let file = File::create(output_path)?;
        let mut writer = BufWriter::new(file);
        self.download_to_write_with_progressbar(filename, &mut writer)
    }

    /// Download a single file from the archive to any [`std::io::Write`], showing a progress bar.
    ///
    /// This method streams the decompressed content directly to the writer without
    /// buffering the entire file in memory.
    ///
    /// Returns the number of bytes written.
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] depending on what happened
    #[cfg(feature = "progressbar")]
    pub fn download_to_write_with_progressbar(
        &self,
        filename: &str,
        writer: &mut dyn std::io::Write,
    ) -> Result<u64, PartialZipError> {
        use indicatif::ProgressBar;

        let mut archive = self.archive.borrow_mut();
        let file = archive.by_name(filename)?;
        let pb = ProgressBar::new(file.compressed_size());
        let bytes_written = io::copy(&mut pb.wrap_read(file), writer)?;
        Ok(bytes_written)
    }

    /// Download multiple files from the archive into memory.
    ///
    /// This method efficiently reuses the underlying connection for all downloads.
    /// Returns a vector of tuples containing the filename and its content.
    ///
    /// **Note:** All file contents are loaded into RAM. For large files, consider using
    /// [`download_multiple_to_dir`](Self::download_multiple_to_dir) instead.
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] on the first file that fails to download.
    /// Successfully downloaded files before the error are not returned.
    pub fn download_multiple(
        &self,
        filenames: &[&str],
    ) -> Result<Vec<(String, Vec<u8>)>, PartialZipError> {
        filenames
            .iter()
            .map(|filename| {
                let content = self.download(filename)?;
                Ok(((*filename).to_string(), content))
            })
            .collect()
    }

    /// Check that filenames don't produce duplicate output basenames when written to a directory
    fn check_duplicate_basenames(filenames: &[&str]) -> Result<(), PartialZipError> {
        let mut seen = std::collections::HashSet::new();
        for filename in filenames {
            let basename = Path::new(filename)
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(filename));
            if !seen.insert(basename.to_owned()) {
                return Err(PartialZipError::IOError(io::Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "duplicate output filename '{}' from archive entry '{filename}'",
                        basename.to_string_lossy()
                    ),
                )));
            }
        }
        Ok(())
    }

    /// Download multiple files from the archive to a directory, streaming to disk.
    ///
    /// This method efficiently reuses the underlying connection for all downloads
    /// and streams each file directly to disk without loading entire contents into memory.
    ///
    /// Files are saved with their original names from the archive. If a file in the archive
    /// contains path separators, only the final component (filename) is used.
    ///
    /// Returns the total number of bytes written across all files.
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] if any file fails to download or if multiple
    /// archive entries would produce the same output filename.
    pub fn download_multiple_to_dir(
        &self,
        filenames: &[&str],
        output_dir: &Path,
    ) -> Result<u64, PartialZipError> {
        Self::check_duplicate_basenames(filenames)?;
        let mut total_bytes = 0u64;
        for filename in filenames {
            // Extract just the filename component (handle paths in archive)
            let output_name = Path::new(filename)
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(filename));
            let output_path = output_dir.join(output_name);
            total_bytes += self.download_to_file(filename, &output_path)?;
        }
        Ok(total_bytes)
    }

    /// Download multiple files from the archive to a directory with progress bars.
    ///
    /// This method efficiently reuses the underlying connection for all downloads
    /// and streams each file directly to disk without loading entire contents into memory.
    ///
    /// Returns the total number of bytes written across all files.
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] on the first file that fails to download.
    #[cfg(feature = "progressbar")]
    pub fn download_multiple_to_dir_with_progressbar(
        &self,
        filenames: &[&str],
        output_dir: &Path,
    ) -> Result<u64, PartialZipError> {
        Self::check_duplicate_basenames(filenames)?;
        let mut total_bytes = 0u64;
        for filename in filenames {
            let output_name = Path::new(filename)
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new(filename));
            let output_path = output_dir.join(output_name);
            total_bytes += self.download_to_file_with_progressbar(filename, &output_path)?;
        }
        Ok(total_bytes)
    }

    /// Returns the options used to create this [`PartialZip`]
    #[must_use]
    pub const fn options(&self) -> &PartialZipOptions {
        &self.options
    }

    /// Get a list of filenames matching a glob pattern (supports `*` and `?`)
    pub fn list_names_matching(&self, pattern: &str) -> Vec<String> {
        self.list_names()
            .into_iter()
            .filter(|name| super::utils::glob_match(pattern, name))
            .collect()
    }

    /// Get a list of files with details matching a glob pattern (supports `*` and `?`)
    pub fn list_detailed_matching(&self, pattern: &str) -> Vec<PartialZipFileDetailed> {
        self.list_detailed()
            .into_iter()
            .filter(|f| super::utils::glob_match(pattern, &f.name))
            .collect()
    }

    /// Download all files matching a glob pattern into memory.
    ///
    /// Returns a vector of tuples containing the filename and its content.
    /// Pattern supports `*` (any sequence) and `?` (any single char).
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] on the first file that fails to download.
    pub fn download_matching(
        &self,
        pattern: &str,
    ) -> Result<Vec<(String, Vec<u8>)>, PartialZipError> {
        let matching: Vec<String> = self.list_names_matching(pattern);
        let refs: Vec<&str> = matching.iter().map(String::as_str).collect();
        self.download_multiple(&refs)
    }

    /// Download all files matching a glob pattern to a directory.
    ///
    /// Pattern supports `*` (any sequence) and `?` (any single char).
    /// Returns the total number of bytes written.
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] on the first file that fails to download.
    pub fn download_matching_to_dir(
        &self,
        pattern: &str,
        output_dir: &Path,
    ) -> Result<u64, PartialZipError> {
        let matching: Vec<String> = self.list_names_matching(pattern);
        let refs: Vec<&str> = matching.iter().map(String::as_str).collect();
        self.download_multiple_to_dir(&refs, output_dir)
    }

    /// Download multiple files in parallel using separate connections.
    ///
    /// Creates up to `max_concurrent` connections to the server and distributes
    /// files across them. Each connection independently fetches and decompresses
    /// its assigned files.
    ///
    /// Returns a vector of tuples containing the filename and its content.
    /// Note: file order in the result may differ from the input order.
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] on the first file that fails to download.
    pub fn download_multiple_parallel(
        &self,
        filenames: &[&str],
        max_concurrent: usize,
    ) -> Result<Vec<(String, Vec<u8>)>, PartialZipError> {
        if filenames.is_empty() {
            return Ok(Vec::new());
        }
        let url = &self.url;
        let options = &self.options;
        let max_concurrent = max_concurrent.max(1).min(filenames.len());

        // Distribute files round-robin across workers
        let mut chunks: Vec<Vec<&str>> = (0..max_concurrent).map(|_| Vec::new()).collect();
        for (i, filename) in filenames.iter().enumerate() {
            chunks[i % max_concurrent].push(filename);
        }

        std::thread::scope(|s| {
            let handles: Vec<_> = chunks
                .into_iter()
                .filter(|c| !c.is_empty())
                .map(|chunk| {
                    s.spawn(move || -> Result<Vec<(String, Vec<u8>)>, PartialZipError> {
                        let pz = Self::new_with_options(url, options)?;
                        chunk
                            .iter()
                            .map(|filename| {
                                let content = pz.download(filename)?;
                                Ok(((*filename).to_string(), content))
                            })
                            .collect()
                    })
                })
                .collect();

            let mut all_results = Vec::with_capacity(filenames.len());
            for handle in handles {
                let results = handle.join().map_err(|_| {
                    PartialZipError::IOError(io::Error::other("download thread panicked"))
                })??;
                all_results.extend(results);
            }
            Ok(all_results)
        })
    }

    /// Download multiple files in parallel to a directory using separate connections.
    ///
    /// Creates up to `max_concurrent` connections and streams each file directly to disk.
    /// Returns the total number of bytes written.
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] on the first file that fails to download.
    pub fn download_multiple_to_dir_parallel(
        &self,
        filenames: &[&str],
        output_dir: &Path,
        max_concurrent: usize,
    ) -> Result<u64, PartialZipError> {
        if filenames.is_empty() {
            return Ok(0);
        }
        Self::check_duplicate_basenames(filenames)?;
        let url = &self.url;
        let options = &self.options;
        let max_concurrent = max_concurrent.max(1).min(filenames.len());

        let mut chunks: Vec<Vec<&str>> = (0..max_concurrent).map(|_| Vec::new()).collect();
        for (i, filename) in filenames.iter().enumerate() {
            chunks[i % max_concurrent].push(filename);
        }

        std::thread::scope(|s| {
            let handles: Vec<_> = chunks
                .into_iter()
                .filter(|c| !c.is_empty())
                .map(|chunk| {
                    s.spawn(move || -> Result<u64, PartialZipError> {
                        let pz = Self::new_with_options(url, options)?;
                        let mut bytes = 0u64;
                        for filename in &chunk {
                            let output_name = Path::new(filename)
                                .file_name()
                                .unwrap_or_else(|| std::ffi::OsStr::new(filename));
                            let output_path = output_dir.join(output_name);
                            bytes += pz.download_to_file(filename, &output_path)?;
                        }
                        Ok(bytes)
                    })
                })
                .collect();

            let mut total_bytes = 0u64;
            for handle in handles {
                total_bytes += handle.join().map_err(|_| {
                    PartialZipError::IOError(io::Error::other("download thread panicked"))
                })??;
            }
            Ok(total_bytes)
        })
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
    max_retries: u32,
    retry_base_delay: Duration,
}

const HTTP_PARTIAL_CONTENT: u32 = 206;

/// Extract content length from a curl Easy handle as u64
fn content_length_as_u64(easy: &mut Easy) -> Result<u64, PartialZipError> {
    let len = easy.content_length_download()?;
    if len >= 0.0 && len.is_finite() {
        // Safety: we've verified len is non-negative and finite
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(len as u64)
    } else {
        Err(std::io::Error::new(ErrorKind::InvalidData, "invalid content length").into())
    }
}

impl PartialReader {
    /// Creates a new [`PartialReader`] with default options
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] enum depending on what happened
    pub fn new(url: &str) -> Result<Self, PartialZipError> {
        Self::new_with_options(url, &PartialZipOptions::default())
    }

    /// Creates a new [`PartialReader`] with range checking option
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] enum depending on what happened
    pub fn new_check_range(url: &str, check_range: bool) -> Result<Self, PartialZipError> {
        Self::new_with_options(url, &PartialZipOptions::default().check_range(check_range))
    }

    /// Creates a new [`PartialReader`] with custom options
    ///
    /// # Errors
    /// Will return a [`PartialZipError`] enum depending on what happened
    pub fn new_with_options(
        url: &str,
        options: &PartialZipOptions,
    ) -> Result<Self, PartialZipError> {
        if !utils::url_is_valid(url) {
            return Err(PartialZipError::InvalidUrl);
        }

        let mut easy = Easy::new();
        easy.url(url)?;
        easy.follow_location(true)?;
        easy.max_redirections(options.max_redirects)?;
        if let Some(timeout) = options.connect_timeout {
            easy.connect_timeout(timeout)?;
        }
        easy.tcp_keepalive(true)?;
        easy.tcp_keepidle(options.tcp_keepidle)?;
        easy.tcp_keepintvl(options.tcp_keepintvl)?;

        // Authentication
        if let Some((username, password)) = &options.basic_auth {
            easy.username(username)?;
            easy.password(password)?;
        }

        // Proxy configuration
        if let Some(proxy_url) = &options.proxy {
            easy.proxy(proxy_url)?;
        }
        if let Some((username, password)) = &options.proxy_auth {
            easy.proxy_username(username)?;
            easy.proxy_password(password)?;
        }

        easy.nobody(true)?;
        easy.write_function(|data| Ok(data.len()))?;
        easy.perform()?;
        let file_size = content_length_as_u64(&mut easy)?;

        if options.check_range {
            // check if range-request is possible by request 1 byte. if 206 Partial Content (HTTP_PARTIAL_CONTENT) is returned, we can make future request.
            easy.range("0-0")?;
            easy.nobody(true)?;
            easy.perform()?;
            let head_size = content_length_as_u64(&mut easy)?;
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
            url: url.to_owned(),
            file_size,
            easy,
            pos: 0,
            max_retries: options.max_retries,
            retry_base_delay: options.retry_base_delay,
        })
    }

    /// Returns the url for the [`PartialReader`]
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }
}

impl io::Read for PartialReader {
    #[allow(clippy::too_many_lines)]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        const MAX_RETRY_DELAY: Duration = Duration::from_secs(60);
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
        let buf_len = buf.len() as u64;
        let maybe_end = start
            .checked_add(buf_len)
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

        let max_attempts = self.max_retries.saturating_add(1);
        let mut last_error = None;

        for attempt in 0..max_attempts {
            if attempt > 0 {
                let delay = self
                    .retry_base_delay
                    .saturating_mul(2u32.saturating_pow(attempt - 1))
                    .min(MAX_RETRY_DELAY);
                log::warn!(
                    "Retrying request (attempt {}/{max_attempts}) after {delay:?}",
                    attempt + 1
                );
                std::thread::sleep(delay);
            }

            let mut written = 0;
            {
                let mut transfer = self.easy.transfer();
                transfer.write_function(|data| {
                    log::trace!("transferred {:x} bytes", data.len());
                    let remaining = buf.len().saturating_sub(written);
                    if data.len() > remaining {
                        log::warn!(
                            "Received {} bytes but only {} bytes remain in destination buffer; aborting transfer",
                            data.len(),
                            remaining
                        );
                        return Ok(0);
                    }
                    if !data.is_empty() {
                        buf[written..written + data.len()].copy_from_slice(data);
                        written += data.len();
                    }
                    Ok(data.len())
                })?;

                match transfer.perform() {
                    Ok(()) => {}
                    Err(e) => {
                        log::warn!("Request failed: {e}");
                        last_error = Some(e);
                        continue;
                    }
                }
            }

            let n = written;
            // new position = position + read amount;
            self.pos = self.pos.checked_add(n as u64).ok_or_else(|| {
                std::io::Error::new(
                    ErrorKind::InvalidData,
                    format!("adding {n} overflows the reader position {}", self.pos),
                )
            })?;
            log::trace!("new self.pos = {:x}", self.pos);
            return Ok(n);
        }

        Err(io::Error::other(format!(
            "request failed after {max_attempts} attempts: {}",
            last_error.expect("at least one attempt was made")
        )))
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
        #[allow(clippy::cast_sign_loss)] // offset >= 0 is checked
        let new_pos = if offset >= 0 {
            // position = base position + offset
            base_pos.checked_add(offset as u64)
        } else {
            // position = base position - |offset|
            base_pos.checked_sub(offset.unsigned_abs())
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
