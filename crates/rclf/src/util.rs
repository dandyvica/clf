//! Utility traits or structs.
use std::fs::{DirEntry, File};
use std::path::PathBuf;

use regex::Regex;

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

pub struct Util;

impl Util {
    // tests whether a `DirEntry` is matching the regex
    fn is_match(entry: &DirEntry, re: &Regex) -> Result<bool, AppError> {
        // converts file name to a string
        let s = entry.path().into_os_string().into_string()?;

        Ok(re.is_match(&s))
    }

    // gives the list of files from a directory, matching the given regex
    fn read_dir(path: &PathBuf, regex: &str) -> Result<Vec<DirEntry>, AppError> {
        // create an empty vector of direntries
        let mut entries: Vec<DirEntry> = Vec::new();

        // create compiled regex
        let re = Regex::new(regex)?;

        // get list of files
        for entry in std::fs::read_dir(path)? {
            if let Ok(entry) = entry {
                if Util::is_match(&entry, &re)? {
                    entries.push(entry);
                }
            }
        }

        Ok(entries)
    }

    // returns the match recent file in the directory `path` and matching the regex
    pub fn most_recent_file(path: &PathBuf, regex: &str) -> Result<Option<PathBuf>, AppError> {
        // get all entries
        let entries = Util::read_dir(path, regex)?;

        // get most recent file according to creation data
        match entries
            .iter()
            .max_by_key(|x| x.metadata().unwrap().created().unwrap())
        {
            None => Ok(None),
            Some(entry) => Ok(Some(entry.path())),
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
    #[cfg(target_os = "linux")]
    fn read_dir() {
        let entries = Util::read_dir(&PathBuf::from("/var/log"), "\\.log$");

        assert!(entries.is_ok());
        assert!(entries.unwrap().len() > 1);
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
