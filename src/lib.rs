#![warn(missing_docs, missing_debug_implementations)]
//! `partialzip` is a crate to download single files from online zip archives or list zip content.
//!
//! The main goal is to save time and memory by only downloading and extracting what you need instead of the whole zip archive.
//!
//! It fetches first the zip data structures and then download only the
//! relevant parts of the archive and proceed to decompress it.
//! # Examples
//! ```no_run
//! use partialzip::PartialZip;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let pz = PartialZip::new(&"https://your_url")?;
//!     let files = pz.list_names();
//!     let content = pz.download(&"yourfile")?;
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
