//! Traits defined here to extend std structs
//!
use std::fs::{read_dir, File};
#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::process::Command;

use log::debug;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::misc::error::{AppCustomErrorKind, AppError, AppResult};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Signature {
    inode: u64,
    dev: u64,
}

/// Tells whether a `PathBuf` is accessible.
pub trait ReadFs {
    fn is_match(self, re: &Regex) -> bool;
    fn is_usable(&self) -> AppResult<()>;
    fn list_files(&self, regex: &str) -> AppResult<Vec<PathBuf>>;
    fn signature(&self) -> AppResult<Signature>;
}

impl ReadFs for PathBuf {
    /// `true` if the path matches the regex
    fn is_match(self, re: &Regex) -> bool {
        // converts file name to a string
        let s = self.into_os_string();
        re.is_match(&s.to_string_lossy())
    }

    /// Tells whether a `PathBuf` is accessible i.e. it combines `has_root()`, `exists()` and `is_file()`.  
    fn is_usable(&self) -> AppResult<()> {
        let _file = File::open(self).map_err(|e| context!(e, "unable to open file {:?}", self))?;

        // if not a file, it's not really usable
        if !self.is_file() {
            Err(AppError::new_custom(
                AppCustomErrorKind::FileNotUsable,
                &format!("path '{:?}' not usable", self),
            ))
        } else {
            Ok(())
        }
    }

    // Gives the list of files from a directory, matching the given regex.
    fn list_files(&self, regex: &str) -> AppResult<Vec<PathBuf>> {
        // create compiled regex
        let re = regex::Regex::new(regex).map_err(|e| context!(e, "error in regex {}", regex))?;

        // get entries
        let entries = read_dir(self)
            .map_err(|e| context!(e, "error trying to read files from {:?} ", self))?;

        // get the list of corresponding files to the regex
        let files: Vec<PathBuf> = entries
            .filter_map(Result::ok) // filter only those result = Ok()
            .filter(|e| e.path().is_match(&re)) // filter only having a path matching the regex
            .map(|e| e.path()) // extract the path from the entry
            .collect();

        Ok(files)
    }

    // get inode and dev from file
    #[cfg(target_family = "unix")]
    fn signature(&self) -> AppResult<Signature> {
        let metadata = self
            .metadata()
            .map_err(|e| context!(e, "error fetching metadata for file {:?} ", self))?;

        Ok(Signature {
            inode: metadata.ino(),
            dev: metadata.dev(),
        })
    }

    #[cfg(target_family = "windows")]
    fn signature(&self) -> AppResult<Signature> {
        unimplemented!("Signature trait not yet implemented for Windows");
    }
}

// Returns the list of files from a command
pub trait ListFiles {
    fn get_file_list(&self) -> AppResult<Vec<PathBuf>>;
}

impl ListFiles for Vec<String> {
    fn get_file_list(&self) -> AppResult<Vec<PathBuf>> {
        // if no data is passed, just return an empty vector
        if self.len() == 0 {
            return Ok(Vec::new());
        }

        // otherwise first element of the vector is the command and rest are arguments
        let cmd = &self[0];
        let args = &self[1..];

        let output = Command::new(&cmd)
            .args(args)
            .output()
            .map_err(|e| {
                context!(
                    e,
                    "unable to read output from command '{:?}' wth args '{:?}'",
                    cmd,
                    args
                )
            })
            .unwrap();

        debug!("cmd={}, args={:?}: returned files={:?}", cmd, args, output);
        let output_as_str = std::str::from_utf8(&output.stdout)
            .map_err(|e| context!(e, "unable to convert '{:?}' to utf8", &output.stdout))?;

        Ok(output_as_str
            .lines()
            .map(PathBuf::from)
            .collect::<Vec<PathBuf>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    #[cfg(target_family = "unix")]
    fn is_usable() {
        assert!(PathBuf::from("foo.txt").is_usable().is_err());
        assert!(PathBuf::from("/var/log/foo.txt").is_usable().is_err());
        assert!(PathBuf::from("/var/log").is_usable().is_err());
        assert!(PathBuf::from("/var/log/syslog").is_usable().is_ok());
    }
    #[test]
    #[cfg(target_family = "windows")]
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
    #[cfg(target_family = "unix")]
    fn is_match() {
        assert!(PathBuf::from("/var/log/kern.log").is_match(&regex::Regex::new("\\.log$").unwrap()));
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn list_files() {
        let entries = PathBuf::from("/var/log").list_files("\\.log$");

        assert!(entries.is_ok());
        assert!(entries.unwrap().len() > 1);
    }
    #[test]
    #[cfg(target_family = "unix")]
    fn signature() {
        let s = PathBuf::from("/var/log").signature();

        assert!(s.is_ok());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn listfiles() {
        let mut cmd = vec![
            "find".to_string(),
            "/var/log".to_string(),
            "-ctime".to_string(),
            "+1".to_string(),
        ];
        let mut files = cmd.get_file_list().unwrap();
        assert!(files.len() > 10);
        assert!(files.iter().all(|f| f.starts_with("/var/log")));

        cmd = vec![
            "bash".to_string(),
            "-c".to_string(),
            "ls /var/log/*.log".to_string(),
        ];
        files = cmd.get_file_list().unwrap();

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
