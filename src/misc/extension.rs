//! Traits defined here to extend Rust standard structures.
use std::fs::{read_dir, File};
use std::io::{BufReader, Read};
#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::process::Command;

#[cfg(target_family = "windows")]
use std::os::windows::prelude::*;

use log::debug;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::misc::error::{AppCustomErrorKind, AppError, AppResult};

// specific linking for Windows signature
#[link(name = r".\src\windows\signature")]
extern "C" {
    fn get_signature_a(file_name: *const u8, signature: *const Signature) -> u32;
    fn get_signature_w(file_name: *const u16, signature: *const Signature) -> u32;
}

#[repr(C)]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
/// A way to uniquely identify a logfile and to know whether is has been archived.
pub struct Signature {
    inode: u64,
    dev: u64,
}

/// All `PathBuf` utility functions.
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
        // first canonicalize path
        let canon = self
            .canonicalize()
            .map_err(|e| context!(e, "unable to canonicalize file {:?}", self))?;
        let _file =
            File::open(&canon).map_err(|e| context!(e, "unable to open file {:?}", self))?;

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
        let mut rc: u32 = 0;
        let sign = Signature::default();

        // convert path to UTF16 Windows string
        let u16_path: Vec<u16> = self.as_os_str().encode_wide().collect();

        println!("signature for {}", self.display());
        println!("u16_path for {}, length={}", String::from_utf16(&u16_path).unwrap(), u16_path.len());

        unsafe {
            rc = get_signature_w(u16_path.as_ptr(), &sign);
        }

        // windows DLL rc should be 0, or rc from GetLastError() API
        if rc != 0 {
            return Err(AppError::new_custom(
                AppCustomErrorKind::WindowsApiError,
                &format!(
                    "Windows API error trying to get file signature = {} for file {}",
                    rc,
                    self.display()
                ),
            ));
        }

        Ok(sign)
    }
}

/// Returns the list of files from a spwand command.
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
                    "unable to read output from command '{:?}' with args '{:?}'",
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
/// When a logfile has a JSOn format, this will be used to read a whole JSON strings, even spanning on several lines.
trait JsonRead {
    fn read_json(&mut self, buf: &mut Vec<u8>) -> AppResult<usize>;
}

impl<R: Read> JsonRead for BufReader<R> {
    fn read_json(&mut self, buf: &mut Vec<u8>) -> AppResult<usize> {
        const LEFT_PARENTHESIS: u8 = 123;
        const RIGHT_PARENTHESIS: u8 = 125;

        let mut left = 0u16;

        for (i, b) in self.bytes().enumerate() {
            let byte = b.map_err(|e| context!(e, "unable to convert value to byte",))?;

            if byte == LEFT_PARENTHESIS {
                buf.push(byte);
                left += 1;
            } else if byte == RIGHT_PARENTHESIS {
                buf.push(byte);
                left -= 1;
            } else if left != 0 {
                buf.push(byte);
            } else {
                continue;
            }

            if left == 0 {
                return Ok(i);
            }
        }

        Ok(0)
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
        let mut cmd = vec![
            "cmd.exe".to_string(),
            "/c".to_string(),
            r"dir /b c:\windows\system32\*.dll".to_string(),
        ];

        let files = cmd.get_file_list().unwrap();
        assert!(files.len() > 10);
        assert!(files
            .iter()
            .all(|f| f.extension().unwrap() == "DLL" || f.extension().unwrap() == "dll"));
    }

    #[test]
    fn json_read() {
        use std::io::Cursor;

        let json = r#"
{"widget": {
    "debug": "on",
    "window": {
        "title": "Sample Konfabulator Widget",
        "name": "main_window",
        "width": 500,
        "height": 500
    },
    "image": { 
        "src": "Images/Sun.png",
        "name": "sun1",
        "hOffset": 250,
        "vOffset": 250,
        "alignment": "center"
    },
    "text": {
        "data": "Click Here",
        "size": 36,
        "style": "bold",
        "name": "text1",
        "hOffset": 250,
        "vOffset": 100,
        "alignment": "center",
        "onMouseUp": "sun1.opacity = (sun1.opacity / 100) * 90;"
    }
}}
{"employees":[
{"name":"Shyam", "email":"shyamjaiswal@gmail.com"},
{"name":"Bob", "email":"bob32@gmail.com"},
{"name":"Jai", "email":"jai87@gmail.com"}
]}
"#;
        let cursor = Cursor::new(json);
        let mut reader = BufReader::new(cursor);
        let mut buffer = Vec::new();

        // read first json
        let ret = reader.read_json(&mut buffer);

        assert!(ret.is_ok());
        let value = ret.unwrap();
        assert_eq!(value, 601);

        let mut one_line = str::replace(&String::from_utf8_lossy(&buffer), "\n", "");
        //println!("oneline={}", one_line);
        assert_eq!(one_line.len(), 576);

        // read next json
        buffer.clear();
        let ret = reader.read_json(&mut buffer);
        assert!(ret.is_ok());
        let value = ret.unwrap();
        assert_eq!(value, 154);

        one_line = str::replace(&String::from_utf8_lossy(&buffer), "\n", "");
        println!("oneline={}", one_line);
        assert_eq!(
            &one_line,
            r#"{"employees":[{"name":"Shyam", "email":"shyamjaiswal@gmail.com"},{"name":"Bob", "email":"bob32@gmail.com"},{"name":"Jai", "email":"jai87@gmail.com"}]}"#
        );
    }
}
