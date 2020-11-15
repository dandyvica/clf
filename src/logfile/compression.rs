//! Manage different types of compression for a logfile
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use xz2::read::XzDecoder;

use crate::misc::error::AppError;

#[serde(rename_all = "lowercase")]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum CompressionScheme {
    Gzip,
    Bzip2,
    Xz,
    Uncompressed,
}

impl CompressionScheme {
    /// True if not compressed
    #[inline(always)]
    pub fn is_compressed(&self) -> bool {
        self != &CompressionScheme::Uncompressed
    }

    /// Creates a new reader depending on the compression. This reader can be used as a regular `BufReader` struct.
    pub fn reader(&self, path: &PathBuf) -> Result<Box<dyn BufRead>, AppError> {
        // open target file
        let file = File::open(&path)?;

        // create a specific reader for each compression scheme
        match &self {
            CompressionScheme::Gzip => {
                let decoder = GzDecoder::new(file);
                let reader = BufReader::new(decoder);
                Ok(Box::new(reader))
            }
            CompressionScheme::Bzip2 => {
                let decoder = BzDecoder::new(file);
                let reader = BufReader::new(decoder);
                Ok(Box::new(reader))
            }
            CompressionScheme::Xz => {
                let decoder = XzDecoder::new(file);
                let reader = BufReader::new(decoder);
                Ok(Box::new(reader))
            }
            CompressionScheme::Uncompressed => {
                let reader = BufReader::new(file);
                Ok(Box::new(reader))
            }
        }
    }
}

/// Conversion from a file extension.
impl From<Option<&str>> for CompressionScheme {
    fn from(ext: Option<&str>) -> Self {
        if ext.is_none() {
            CompressionScheme::Uncompressed
        } else {
            match ext.unwrap() {
                "gz" => CompressionScheme::Gzip,
                "bz2" => CompressionScheme::Bzip2,
                "xz" => CompressionScheme::Xz,
                _ => CompressionScheme::Uncompressed,
            }
        }
    }
}