#![warn(missing_docs, missing_debug_implementations)]
//! `partialzip` is a crate to download single files from online zip archives or list zip content.
//!
//! The main goal is to save time and memory by only downloading and extracting what you need
//! instead of the whole zip archive.
//!
//! It fetches first the zip data structures and then downloads only the
//! relevant parts of the archive and proceeds to decompress it.
//!
//! # Basic Example
//!
//! ```no_run
//! use partialzip::PartialZip;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let pz = PartialZip::new(&"https://example.com/archive.zip")?;
//!     let files = pz.list_names();
//!     let content = pz.download("myfile.txt")?;
//!     Ok(())
//! }
//! ```
//!
//! # Streaming Large Files
//!
//! For large files, use [`PartialZip::download_to_file`] or [`PartialZip::download_to_write`]
//! to stream directly to disk without loading the entire file into memory:
//!
//! ```no_run
//! use partialzip::PartialZip;
//! use std::path::Path;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let pz = PartialZip::new(&"https://example.com/archive.zip")?;
//!
//!     // Stream directly to a file - recommended for large files
//!     let bytes_written = pz.download_to_file("large_file.bin", Path::new("output.bin"))?;
//!     println!("Downloaded {} bytes", bytes_written);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Configuration Options
//!
//! Use [`PartialZipOptions`] to configure connection behavior:
//!
//! ```no_run
//! use partialzip::{PartialZip, PartialZipOptions};
//! use std::time::Duration;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let options = PartialZipOptions::new()
//!         .max_redirects(5)                              // Limit redirects (default: 10)
//!         .connect_timeout(Some(Duration::from_secs(60))) // Connection timeout (default: 30s)
//!         .check_range(true);                            // Verify server supports range requests
//!
//!     let pz = PartialZip::new_with_options(&"https://example.com/archive.zip", options)?;
//!     Ok(())
//! }
//! ```
/// Core module for the partialzip crate
pub mod partzip;
pub use partzip::PartialReader;
pub use partzip::PartialZip;
pub use partzip::PartialZipCompressionMethod;
pub use partzip::PartialZipError;
pub use partzip::PartialZipFileDetailed;
pub use partzip::PartialZipOptions;
pub use partzip::DEFAULT_CONNECT_TIMEOUT_SECS;
pub use partzip::DEFAULT_MAX_REDIRECTS;
pub use partzip::DEFAULT_TCP_KEEPIDLE_SECS;
pub use partzip::DEFAULT_TCP_KEEPINTVL_SECS;
/// Small utilities mostly for URLs
mod utils;

mod tests;
