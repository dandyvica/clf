//! Utility traits or structs.
use std::fs::File;
use std::path::PathBuf;
use std::process::Command;

//use regex::Regex;

use crate::misc::error::{AppCustomErrorKind, AppError};

/// Gather all constants in a single struct
pub struct Cons;

impl Cons {
    /// A default value for the retention of data in the snapshot file.
    pub const DEFAULT_RETENTION: u64 = 86000 * 7;

    /// Variable name prefix to be inserted for each variable.
    pub const VAR_PREFIX: &'static str = "CLF_";

    /// Default capacity for all `Vec` or `HashMap` pre-allocations
    pub const DEFAULT_CONTAINER_CAPACITY: usize = 30;

    /// Default capacity for all strings pre-allocations
    pub const DEFAULT_STRING_CAPACITY: usize = 1024;

    /// We define here the maximum size for the logger file (in Mb).
    pub const MAX_LOGGER_SIZE: u64 = 50;

    /// Returns the number of seconds for a standard timeout when not defined in the YAML file.
    /// Needed by `serde`.
    pub const fn default_timeout() -> u64 {
        180
    }
}

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
            Err(AppError::new(
                AppCustomErrorKind::NotAFile,
                &format!(
                    "file: {} is not a file, probably a directory",
                    self.display()
                ),
            ))
        } else {
            Ok(())
        }
    }
}

/// Gather all utility methods into a single struct
pub struct Util;

impl Util {
    /// Tests whether a `DirEntry` is matching the regex.
    #[cfg(test)]
    fn is_match(entry: &std::fs::DirEntry, re: &regex::Regex) -> Result<bool, AppError> {
        // converts file name to a string
        let s = entry.path().into_os_string().into_string()?;

        Ok(re.is_match(&s))
    }

    // Gives the list of files from a directory, matching the given regex.
    #[cfg(test)]
    fn read_dir(path: &PathBuf, regex: &str) -> Result<Vec<std::fs::DirEntry>, AppError> {
        // create an empty vector of direntries
        let mut entries: Vec<std::fs::DirEntry> = Vec::new();

        // create compiled regex
        let re = regex::Regex::new(regex)?;

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

    /// Returns the match recent file in the directory `path` and matching the regex.
    // pub fn most_recent_file(path: &PathBuf, regex: &str) -> Result<Option<PathBuf>, AppError> {
    //     // get all entries
    //     let entries = Util::read_dir(path, regex)?;

    //     // get most recent file according to creation data
    //     match entries
    //         .iter()
    //         .max_by_key(|x| x.metadata().unwrap().created().unwrap())
    //     {
    //         None => Ok(None),
    //         Some(entry) => Ok(Some(entry.path())),
    //     }
    // }

    /// Spawns a command and returns a list of file names corresponding to the command.
    pub fn get_list(cmd: &str, args: Option<&[String]>) -> Result<Vec<PathBuf>, AppError> {
        let output = match args {
            None => Command::new(&cmd).output()?,
            Some(_args) => Command::new(&cmd).args(_args).output()?,
        };

        //debug!("output={:?}", output);
        let output_as_str = std::str::from_utf8(&output.stdout)?;

        Ok(output_as_str
            .lines()
            .map(PathBuf::from)
            .collect::<Vec<PathBuf>>())
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

    #[test]
    #[cfg(target_os = "linux")]
    fn list_files_find() {
        let files = Util::get_list(
            &"find",
            Some(&[
                "/var/log".to_string(),
                "-ctime".to_string(),
                "+1".to_string(),
            ]),
        )
        .expect("error listing files");
        assert!(files.len() > 10);
        assert!(files.iter().all(|f| f.starts_with("/var/log")));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn list_files_ls() {
        let files = Util::get_list(
            &"bash",
            Some(&["-c".to_string(), "ls /var/log/*.log".to_string()]),
        )
        .expect("error listing files");
        assert!(files.len() > 10);
        assert!(files.iter().all(|f| f.starts_with("/var/log")));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn list_files_shell() {
        let files = Util::get_list(
            &"cmd.exe",
            Some(&[
                "/c".to_string(),
                r"dir /b c:\windows\system32\*.dll".to_string(),
            ]),
        )
        .expect("error listing files");
        //println!("{:?}", files);
        assert!(files.len() > 1000);
        //assert!(files.iter().all(|f| f.ends_with("dll")));
    }
}
