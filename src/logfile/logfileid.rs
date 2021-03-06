//! A structure representing a logfile identification, with all its usual attributes (path, extension, ...).
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::context;
use crate::logfile::compression::CompressionScheme;
use crate::misc::error::{AppError, AppResult};
use crate::misc::extension::{ReadFs, Signature};

/// Logfile variable fields that change depending on the path.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LogFileID {
    /// File & path as a `PathBuf`.
    pub declared_path: PathBuf,

    /// PathBuf that has been canonicalized (e.g.: symlinks resolved)
    pub canon_path: PathBuf,

    /// Directory part or `None` if not existing.
    pub directory: Option<PathBuf>,

    /// Extension or `None` if no extension.
    pub extension: Option<String>,

    /// Compression method
    pub compression: CompressionScheme,

    /// Uniquely identifies a logfile
    pub signature: Signature,
}

impl LogFileID {
    /// Fill all variable fields from declared
    #[cfg(test)]
    #[cfg(target_family = "unix")]
    pub fn from_declared<P: AsRef<Path>>(path: P, hash_buffer_size: usize) -> AppResult<Self> {
        let mut id = LogFileID::default();
        id.update(path, hash_buffer_size)?;

        Ok(id)
    }

    /// Update some logfile fields with up to date path values. This is used when detecting rotation for logfiles
    pub fn update<P: AsRef<Path>>(&mut self, path: P, hash_buffer_size: usize) -> AppResult<()> {
        // check if we can really use the file
        self.declared_path = PathBuf::from(path.as_ref());

        // canonicalize path: absolute form of the path with all intermediate
        // components normalized and symbolic links resolved.
        let canon = self
            .declared_path
            .canonicalize()
            .map_err(|e| context!(e, "unable to canonicalize file:{:?}", &self.declared_path))?;

        self.directory = canon.parent().map(|p| p.to_path_buf());
        self.extension = canon.extension().map(|x| x.to_string_lossy().to_string());
        self.compression = CompressionScheme::from(self.extension.as_deref());

        // // get inode & dev ID
        self.signature = canon.signature(hash_buffer_size)?;
        trace!(
            "current signature for {:?} is {:?}",
            &canon,
            &self.signature
        );

        // finally save path
        self.canon_path = canon;

        Ok(())
    }
}

#[cfg(test)]
#[cfg(target_family = "unix")]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn id() {
        // let id = LogFileID::from_declared("/lib/ld-linux.so.2", 4096).unwrap();
        // assert_eq!(
        //     id.canon_path,
        //     PathBuf::from("/lib/i386-linux-gnu/ld-2.31.so")
        // );

        let id = LogFileID::from_declared("/foo", 4096);
        assert!(id.is_err());
    }
}
