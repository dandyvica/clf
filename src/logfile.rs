//! A structure representing a logfile, with all related attributes.
use std::ffi::OsString;
use std::fs::Metadata;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;

use serde::{Deserialize, Serialize};

use crate::error::{AppCustomErrorKind, AppError};
use crate::util::Usable;

/// A wrapper to get logfile information and and related attributes.
#[derive(Debug, Serialize, Deserialize)]
pub struct LogFile {
    /// file & path as a PathBuf
    pub path: PathBuf,

    /// extension as an OsString (owned) or None if no extension
    pub extension: Option<String>,

    /// platform specific metadata
    // #[serde(skip_serializing)]
    pub compressed: bool,

    /// position of the last run. Used to seek the file pointer to this point.
    pub last_pos: u64,

    /// last line number during the last search
    pub last_line: u64,

    /// last time logfile is processed
    pub last_run: u64,

    /// Linux inode or Windows equivalent
    pub inode: u64,

    // Linux device ID or equivalent for Windows
    pub dev: u64,
}

impl LogFile {
    /// Creates a `LogFile` by providing the full logfile path. It also sets platform specific features
    /// like file *inode* or *dev*. The file path is checked for accessibility and is canonicalized.
    ///
    /// Examples:
    ///
    /// ```rust
    /// use clf::logfile::LogFile;
    ///
    /// let lf_ok = LogFile::new("/etc/hosts.allow").unwrap();
    /// assert_eq!(lf_ok.path.to_str(), Some("/etc/hosts.allow"));
    /// assert_eq!(lf_ok.extension.unwrap(), "allow");
    /// ```
    pub fn new<P: AsRef<Path>>(file_name: P) -> Result<LogFile, AppError> {
        // check if we can really use the file
        let path = PathBuf::from(file_name.as_ref());
        let extension = path
            .extension()
            .and_then(|x| Some(x.to_string_lossy().to_string()));

        if !path.is_usable() {
            return Err(AppError::App {
                err: AppCustomErrorKind::FileNotUsable,
                msg: format!("file {:?} is not usable", path),
            });
        }

        const COMPRESSED_EXT: &'static [&'static str] = &["gz", "zip", "xz"];
        let compressed = match &extension {
            None => false,
            Some(s) => COMPRESSED_EXT.contains(&s.as_str()),
        };

        // canonicalize path: absolute form of the path with all intermediate
        // components normalized and symbolic links resolved.
        let canon = path.canonicalize()?;

        // get metadata if possible
        let metadata = path.metadata()?;

        // calculate number of seconds since EPOCH
        let time =  SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;

        // get inode & dev ID
        let mut inode = 0u64;
        let mut dev = 0u64;

        // inode & dev are platform specific
        if cfg!(target_os = "linux") {
            inode = metadata.ino();
            dev = metadata.dev();
        }

        Ok(LogFile {
            path: canon,
            extension: extension,
            compressed: compressed,
            last_pos: 0u64,
            last_line: 0u64,
            last_run: time.as_secs(),
            inode: inode,
            dev: dev,
        })
    }
}

/// Two log files are considered equal if they have the same name, inode & dev
impl PartialEq for LogFile {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.dev == other.dev && self.inode == self.inode
    }
}

#[cfg(test)]
mod tests {
    use crate::error::*;
    use crate::logfile::LogFile;

    use serde::{Deserialize, Serialize};

    //#[test]
    // #[cfg(target_os = "linux")]
    // fn test_new() {
    //     let mut lf_ok = LogFile::new("/usr/bin/zip").build().unwrap();
    //     assert_eq!(lf_ok.path.to_str(), Some("/usr/bin/zip"));
    //     assert!(lf_ok.extension.is_none());

    //     lf_ok = LogFile::new("/etc/hosts.allow").build().unwrap();
    //     assert_eq!(lf_ok.path.to_str(), Some("/etc/hosts.allow"));
    //     assert_eq!(lf_ok.extension.unwrap(), "allow");

    //     // file not found
    //     let mut lf_err = LogFile::new("/usr/bin/foo").build();
    //     assert!(lf_err.is_err());
    //     match lf_err.unwrap_err() {
    //         AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomError::FileNotAccessible),
    //         _ => panic!("error not expected here!"),
    //     };

    //     // not a file
    //     lf_err = LogFile::new("/usr/bin").build();
    //     match lf_err.unwrap_err() {
    //         AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomError::NotAFile),
    //         _ => panic!("error not expected here!"),
    //     };

    //     // file has no root
    //     lf_err = LogFile::new("usr/bin/foo").build();
    //     assert!(lf_err.is_err());
    //     match lf_err.unwrap_err() {
    //         AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomError::FileHasNoRoot),
    //         _ => panic!("error not expected here!"),
    //     };
    // }

    // #[test]
    // fn test_deserialize() {
    //     #[derive(Serialize, Deserialize)]
    //     struct Data {
    //         list: Vec<LogFile>,
    //     }

    //     let data = r#"
    //     {
    //        "list": [
    //             {
    //                 "path": "/usr/bin/zip",
    //                 "last_pos": 0,
    //                 "inode": 0,
    //                 "dev": 0
    //             },
    //             {
    //                 "path": "/etc/hosts.allow",
    //                 "last_pos": 1,
    //                 "inode": 1,
    //                 "dev": 1
    //             }
    //         ]
    //     }
    //     "#;

    //     let json: Data = serde_json::from_str(data).unwrap();

    //     assert_eq!(json.list[0].path.to_str(), Some("/usr/bin/zip"));
    //     assert_eq!(json.list[0].last_pos, 0u64);
    //     assert_eq!(json.list[0].inode, 0u64);
    //     assert_eq!(json.list[0].dev, 0u64);

    //     assert_eq!(json.list[1].path.to_str(), Some("/etc/hosts.allow"));
    //     assert_eq!(json.list[1].last_pos, 1u64);
    //     assert_eq!(json.list[1].inode, 1u64);
    //     assert_eq!(json.list[1].dev, 1u64);
    // }

    // #[test]
    // #[cfg(target_os = "windows")]
    // fn test_new() {
    //     let lf_ok = LogFile::new(r"C:\Windows\System32\cmd.exe", true).unwrap();
    //     assert_eq!(lf_ok.path.to_str(), Some(r"C:\Windows\System32\cmd.exe"));
    //     assert_eq!(lf_ok.extension.unwrap(), "exe");

    //     // file not found
    //     let mut lf_err = LogFile::new(r"C:\Windows\System32\foo.exe", true);
    //     assert!(lf_err.is_err());
    //     match lf_err.unwrap_err() {
    //         AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomError::FileNotAccessible),
    //         _ => panic!("error not expected here!"),
    //     };

    //     // not a file
    //     lf_err = LogFile::new(r"C:\Windows\System32", true);
    //     assert!(lf_err.is_err());
    //     match lf_err.unwrap_err() {
    //         AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomError::NotAFile),
    //         _ => panic!("error not expected here!"),
    //     };

    //     // file has no root
    //     lf_err = LogFile::new(r"Windows\System32\cmd.exe", true);
    //     assert!(lf_err.is_err());
    //     match lf_err.unwrap_err() {
    //         AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomError::FileHasNoRoot),
    //         _ => panic!("error not expected here!"),
    //     };
    // }

    // #[test]
    // fn test_search() {
    //     let mut logfile = LogFile::new("./tests/files/access.log", false).unwrap();

    //     let mut re = Regex::new("^83").unwrap();
    //     let matched: bool = logfile.search(&re).unwrap().unwrap();
    //     assert!(matched);

    //     re = Regex::new(r"^83.(\d+).(\d+).(\d+)").unwrap();
    //     let matched: Vec<String> = logfile.search(&re).unwrap().unwrap();
    //     assert_eq!(matched, vec!["83.167.113.100", "167", "113", "100"]);
    // }

    // #[test]
    // fn test_basic_search() {
    //     let mut logfile = LogFile::new("./tests/files/simple.txt", false).unwrap();

    //     let re = Regex::new("^B").unwrap();
    //     let matched: Option<bool> = logfile.search(&re).unwrap();
    //     assert!(matched.is_none());

    //     assert_eq!(logfile.last_pos, 110);
    // }
}
