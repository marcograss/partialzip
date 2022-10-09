#![warn(missing_docs, missing_debug_implementations)]
//! `partialzip` is a crate to download single files from online zip archives or list zip content.
//!
//! The main goal is to save time and memory by only downloading and extracting what you need instead of the whole zip archive.
//!
//! It fetches first the zip data structures and then download only the
//! relevant parts of the archive and proceed to decompress it.
/// Core module for the partialzip crate
pub mod partzip;
/// Small utilities mostly for URLs
pub mod utils;

mod tests;
