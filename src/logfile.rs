//! A structure representing a logfile, with all its related attributes. Those attributes are
//! coming from the processing of the log file, every time it's read to look for patterns.
use std::path::{Path, PathBuf};
//use std::time::{Instant, SystemTime};

#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;

use serde::{Deserialize, Serialize};

use crate::error::{AppCustomErrorKind, AppError};
use crate::util::Usable;

/// A wrapper to store log file processing data.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RunData {
    /// tag name
    pub tag: String,

    /// position of the last run. Used to seek the file pointer to this point.
    pub last_offset: u64,

    /// last line number during the last search
    pub last_line: u64,

    /// last time logfile is processed
    pub last_run: u64,
}

/// A wrapper to get logfile information and its related attributes.
#[derive(Debug, Serialize, Deserialize)]
pub struct LogFile {
    /// file & path as a `PathBuf`
    pub path: PathBuf,

    /// directory part or `None` if not existing
    pub directory: Option<PathBuf>,

    /// extension or `None` if no extension
    pub extension: Option<String>,

    /// `true` if logfile is compressed
    pub compressed: bool,

    /// Linux inode or Windows equivalent
    pub inode: u64,

    /// Linux device ID or equivalent for Windows
    pub dev: u64,

    /// Run time data are stored each time a logfile is searched
    pub rundata: Vec<RunData>,
}

impl LogFile {
    /// Creates a `LogFile` by providing the full logfile path. It also sets platform specific features
    /// like file *inode* or *dev*. The file path is checked for accessibility and is canonicalized. It also
    /// contains run time data, which correspond to the data created each time a logfile instance is searched
    /// for patterns.
    pub fn new<P: AsRef<Path>>(file_name: P) -> Result<LogFile, AppError> {
        // check if we can really use the file
        let path = PathBuf::from(file_name.as_ref());
        let directory = path.parent().and_then(|p| Some(p.to_path_buf()));
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
        //let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;

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
            directory: directory,
            extension: extension,
            compressed: compressed,
            inode: inode,
            dev: dev,
            rundata: Vec::new(),
        })
    }

    /// Returns the list of tags of this `LogFile`.
    pub fn tags(&self) -> Vec<&str> {
        let v: Vec<_> = self.rundata.iter().map(|x| x.tag.as_str()).collect();
        v
    }

    /// Returns `true` if `tag_name` is found in this `LogFile`.
    pub fn contains(&self, tag_name: &str) -> bool {
        match self.rundata.iter().find(|x| x.tag == tag_name) {
            Some(_) => true,
            None => false,
        }
    }

    /// Pushes a new `RunData` structure into the logfile.
    pub fn push(&mut self, data: RunData) {
        self.rundata.push(data);
    }

    /// Returns an Option on a reference of a `RunData`, mapping the first
    /// tag name passed in argument.
    pub fn get_mut(&mut self, tag_name: &str) -> Option<&mut RunData> {
        self.rundata.iter_mut().find(|x| x.tag == tag_name)
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
    use crate::logfile::{LogFile, RunData};

    //use serde::{Deserialize, Serialize};

    // useful set of data for our unit tests
    const JSON: &'static str = r#"
       [
            {
                "path": "/usr/bin/zip",
                "compressed": false, 
                "inode": 1,
                "dev": 1,
                "rundata": [
                    {
                        "tag": "tag1",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000
                    },
                    {
                        "tag": "tag2",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000
                    }
                ]
            },
            {
                "path": "/etc/hosts.allow",
                "compressed": false, 
                "inode": 1,
                "dev": 1,
                "rundata": [
                    {
                        "tag": "tag3",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000
                    },
                    {
                        "tag": "tag3",
                        "last_offset": 1500,
                        "last_line": 10,
                        "last_run": 1000000
                    }
                ]
            }
        ]
    "#;

    fn load_json() -> Vec<LogFile> {
        serde_json::from_str(JSON).unwrap()
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_new() {
        let mut lf_ok = LogFile::new("/usr/bin/zip").unwrap();
        assert_eq!(lf_ok.path.to_str(), Some("/usr/bin/zip"));
        assert!(lf_ok.extension.is_none());

        lf_ok = LogFile::new("/etc/hosts.allow").unwrap();
        assert_eq!(lf_ok.path.to_str(), Some("/etc/hosts.allow"));
        assert_eq!(lf_ok.extension.unwrap(), "allow");

        // file not found
        let mut lf_err = LogFile::new("/usr/bin/foo");
        assert!(lf_err.is_err());
        match lf_err.unwrap_err() {
            AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomErrorKind::FileNotUsable),
            _ => panic!("error not expected here!"),
        };

        // not a file
        lf_err = LogFile::new("/usr/bin");
        match lf_err.unwrap_err() {
            AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomErrorKind::FileNotUsable),
            _ => panic!("error not expected here!"),
        };

        // file has no root
        lf_err = LogFile::new("usr/bin/foo");
        assert!(lf_err.is_err());
        match lf_err.unwrap_err() {
            AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomErrorKind::FileNotUsable),
            _ => panic!("error not expected here!"),
        };
    }

    #[test]
    fn test_deserialize() {
        let json = crate::logfile::tests::load_json();

        assert_eq!(json[0].path.to_str(), Some("/usr/bin/zip"));
        assert_eq!(json[1].path.to_str(), Some("/etc/hosts.allow"));
    }

    #[test]
    fn test_push() {
        let mut json = crate::logfile::tests::load_json();

        let rundata = RunData {
            tag: "another_tag".to_string(),
            ..Default::default()
        };
        json[0].push(rundata);

        assert_eq!(json[0].rundata.len(), 3);
        assert_eq!(json[0].tags(), vec!["tag1", "tag2", "another_tag"]);
        assert!(json[0].contains("tag1"));
        assert!(!json[0].contains("tag3"));
    }

    #[test]
    fn test_getmut() {
        let mut json = crate::logfile::tests::load_json();
        assert_eq!(json[1].rundata.len(), 2);

        // tag4 is not part of LogFile
        let mut rundata = json[1].get_mut("tag4");
        assert!(rundata.is_none());

        // but tag3 is and even is duplicated
        rundata = json[1].get_mut("tag3");
        assert!(rundata.is_some());
        rundata.unwrap().last_line = 999;

        assert_eq!(json[1].rundata[0].last_line, 999);
    }

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
}
