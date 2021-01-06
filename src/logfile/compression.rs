//! Manage different types of compression for a logfile.
use serde::{Deserialize, Serialize};

#[serde(rename_all = "lowercase")]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
/// A list of possible compression methods for a logfile.
pub enum CompressionScheme {
    Gzip,
    Bzip2,
    Xz,
    Uncompressed,
}

impl CompressionScheme {
    /// True if not compressed
    #[allow(dead_code)]
    #[inline(always)]
    pub fn is_compressed(&self) -> bool {
        self != &CompressionScheme::Uncompressed
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

/// By default, no compression
impl Default for CompressionScheme {
    fn default() -> Self {
        CompressionScheme::Uncompressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compression() {
        assert_eq!(
            CompressionScheme::from(None),
            CompressionScheme::Uncompressed
        );
        assert_eq!(
            CompressionScheme::from(Some("foo")),
            CompressionScheme::Uncompressed
        );
        assert_eq!(
            CompressionScheme::from(Some("")),
            CompressionScheme::Uncompressed
        );
        assert_eq!(CompressionScheme::from(Some("gz")), CompressionScheme::Gzip);
        assert_eq!(CompressionScheme::from(Some("xz")), CompressionScheme::Xz);
        assert_eq!(
            CompressionScheme::from(Some("bz2")),
            CompressionScheme::Bzip2
        );
    }
}
