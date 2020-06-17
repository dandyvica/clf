//! Utility traits or structs.
use std::fs::File;
use std::path::PathBuf;

use crate::error::{AppCustomErrorKind, AppError};

/// Default capacity for all `Vec` or `HashMap` pre-allocations
pub const DEFAULT_CONTAINER_CAPACITY: usize = 30;

/// Default capacity for all strings pre-allocations
pub const DEFAULT_STRING_CAPACITY: usize = 1024;

/// Tells whether a `PathBuf` is accessible.
pub trait Usable {
    fn is_usable(&self) -> Result<(), AppError>;
}

impl Usable for PathBuf {
    /// Tells whether a `PathBuf` is accessible i.e. it combines `has_root()`, `exists()` and `is_file()`.  
    fn is_usable(&self) -> Result<(), AppError> {
        //self.has_root() && self.exists() && self.is_file()
        //self.exists() && self.is_file()
        let _ = File::open(self)?;

        // need to check if this is really a file
        if !self.is_file() {
            Err(AppError::App {
                err: AppCustomErrorKind::NotAFile,
                msg: format!(
                    "file: {} is not a file, probably a directory",
                    self.display()
                ),
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn is_usable() {
        assert!(PathBuf::from("foo.txt").is_usable().is_err());
        assert!(PathBuf::from("/var/log/foo.txt").is_usable().is_err());
        assert!(PathBuf::from("/var/log").is_usable().is_err());
        assert!(PathBuf::from("/var/log/syslog").is_usable().is_ok());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn is_usable() {
        assert!(PathBuf::from("foo.txt").is_usable().is_err());
        assert!(PathBuf::from(r"c:\windows\system32\foo.txt")
            .is_usable()
            .is_err());
        assert!(PathBuf::from(r"c:\windows\system32").is_usable().is_err());
        assert!(PathBuf::from(r"c:\windows\system32\cmd.exe")
            .is_usable()
            .is_ok());
    }
}
