//! A structure representing a logfile, with all its related attributes. Those attributes are
//! coming from the processing of the log file, every time it's read to look for patterns.
use log::{debug, info};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
//use std::time::{Instant, SystemTime};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::time::SystemTime;

#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;

use flate2::read::{GzDecoder, ZlibDecoder};
use serde::{Deserialize, Serialize};

use crate::config::Tag;
use crate::error::{AppCustomErrorKind, AppError};
//use crate::logfile::logfile::{LogFile, RunData};
use crate::pattern::PatternSet;

use crate::util::Usable;

/// A wrapper to store log file processing data.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RunData {
    /// tag name
    name: String,

    /// position of the last run. Used to seek the file pointer to this point.
    last_offset: u64,

    /// last line number during the last search
    last_line: u64,

    /// last time logfile is processed
    last_run: u64,
}

/// A wrapper to get logfile information and its related attributes.
#[derive(Debug, Serialize, Deserialize)]
pub struct LogFile {
    /// File & path as a `PathBuf`.
    path: PathBuf,

    /// Directory part or `None` if not existing.
    directory: Option<PathBuf>,

    /// Extension or `None` if no extension.
    extension: Option<String>,

    /// `true` if logfile is compressed.
    compressed: bool,

    /// Linux inode or Windows equivalent.
    inode: u64,

    /// Linux device ID or equivalent for Windows.
    dev: u64,

    /// Run time data that are stored each time a logfile is searched for patterns.
    rundata: HashMap<String, RunData>,
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
            rundata: HashMap::new(),
        })
    }

    /// Returns the list of tags of this `LogFile`.
    pub fn tags(&self) -> Vec<&str> {
        self.rundata
            .keys()
            .map(|x| x.as_str())
            .collect::<Vec<&str>>()
    }

    /// Returns `true` if `name` is found in this `LogFile`.
    pub fn contains_key(&self, name: &str) -> bool {
        self.rundata.contains_key(name)
    }

    /// Returns an Option on a reference of a `RunData`, mapping the first
    /// tag name passed in argument.
    pub fn or_insert(&mut self, name: &str) -> &mut RunData {
        self.rundata.entry(name.to_string()).or_insert(RunData {
            name: name.to_string(),
            ..Default::default()
        })
    }
}

/// Two log files are considered equal if they have the same name, inode & dev.
impl PartialEq for LogFile {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.dev == other.dev && self.inode == self.inode
    }
}

/// Trait which provides a seek function, and is implemented for all
/// `BufReader<T>` types used in `Lookup` trait.
pub trait Seeker {
    /// Simulates the `seek`method for all used `BufReader<R>`.
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError>;
}

// impl<T: Seek> Seeker for BufReader<T> {
//     fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
//         self.seek(SeekFrom::Start(offset))
//             .map_err(|e| AppError::Io(e))
//     }
// }

impl Seeker for BufReader<File> {
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
        self.seek(SeekFrom::Start(offset))
            .map_err(|e| AppError::Io(e))
    }
}

impl Seeker for BufReader<GzDecoder<File>> {
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
        // if 0, nothing to do
        if offset == 0 {
            return Ok(0);
        }

        let pos = match self.by_ref().bytes().nth((offset - 1) as usize) {
            None => {
                return Err(AppError::App {
                    err: AppCustomErrorKind::SeekPosBeyondEof,
                    msg: format!("tried to set offset beyond EOF, at offset: {}", offset),
                })
            }
            Some(x) => x,
        };
        Ok(pos.unwrap() as u64)
    }
}

// impl Seeker for BufReader<ZlibDecoder<File>> {
//     fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
//         let pos = match self.by_ref().bytes().nth((offset - 1) as usize) {
//             None => {
//                 return Err(AppError::App {
//                     err: AppCustomErrorKind::SeekPosBeyondEof,
//                     msg: format!("tried to set offset beyond EOF, at: {}", offset),
//                 })
//             }
//             Some(x) => x,
//         };
//         Ok(pos.unwrap() as u64)
//     }
// }

/// Trait, implemented by `LogFile` to search patterns.
pub trait Lookup {
    fn lookup(&mut self, tag: &Tag) -> Result<(), AppError>;
    fn lookup_from_reader<R: BufRead + Seeker>(
        &mut self,
        reader: R,
        tag: &Tag,
    ) -> Result<(), AppError>;
}

impl Lookup for LogFile {
    ///Just a wrapper function for a file.
    fn lookup(&mut self, tag: &Tag) -> Result<(), AppError> {
        // open target file
        let file = File::open(&self.path)?;

        // if file is compressed, we need to call a specific reader
        let reader = match self.extension.as_deref() {
            Some("gz") => {
                let decoder = GzDecoder::new(file);
                let reader = BufReader::new(decoder);
                self.lookup_from_reader(reader, tag)?;
            }
            // Some("zip") => {
            //     let decoder = ZlibDecoder::new(file);
            //     let reader = BufReader::new(decoder);
            //     self.lookup_from_reader(reader, tag, settings)?;
            // },
            Some(&_) | None => {
                let reader = BufReader::new(file);
                self.lookup_from_reader(reader, tag)?;
            }
        };

        // if self.compressed {
        //     info!("file {:?} is compressed", &self.path);
        //     let decoder = GzDecoder::new(file);
        //     let reader = BufReader::new(decoder);
        //     self.lookup_from_reader(reader, tag, settings)?;
        // } else {
        //     let reader = BufReader::new(file);
        //     self.lookup_from_reader(reader, tag, settings)?;
        // };

        //output
        Ok(())
    }

    fn lookup_from_reader<R: BufRead + Seeker>(
        &mut self,
        mut reader: R,
        tag: &Tag,
    ) -> Result<(), AppError> {
        // uses the same buffer
        let mut line = String::with_capacity(1024);

        // get rundata corresponding to tag name
        let mut rundata = self.or_insert(&tag.name);

        // initialize counters
        info!(
            "starting read from last offset={}, last line={}",
            rundata.last_offset, rundata.last_line
        );
        let mut bytes_count = rundata.last_offset;
        let mut line_number = rundata.last_line;

        // move to position if already recorded, and not rewind
        //if !tag.options.rewind && rundata.last_offset != 0 {
        if rundata.last_offset != 0 {
            reader.set_offset(rundata.last_offset)?;
        }

        loop {
            // read until \n (which is included in the buffer)
            let ret = reader.read_line(&mut line);

            // read_line() returns a Result<usize>
            match ret {
                Ok(bytes_read) => {
                    // EOF: save last file address to restart from this address for next run
                    if bytes_read == 0 {
                        //self.last_offset = reader.seek(SeekFrom::Current(0)).unwrap();
                        break;
                    }

                    // we've been reading a new line successfully
                    line_number += 1;
                    bytes_count += bytes_read as u64;
                    //println!("====> line#={}, file={}", line_number, line);

                    // check. if somethin found
                    // if let Some(caps) = tag.patterns.captures(&line) {
                    //     debug!("file {:?}, line match: {:?}", self.path, caps);
                    //     break;

                    //     // if option.script, replace capture groups and call script
                    //     // time out if any,
                    // }
                    if let Some(caps) = tag.captures(&line) {
                        debug!("line match: {:?}", caps);
                        break;
                    }

                    // reset buffer to not accumulate data
                    line.clear();
                }
                // a rare IO error could occur here
                Err(err) => {
                    return Err(AppError::Io(err));
                }
            };
        }

        // save current offset and line number
        rundata.last_offset = bytes_count;
        rundata.last_line = line_number;

        // and last run
        let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        rundata.last_run = time.as_secs();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::error::*;
    use crate::logfile::{LogFile, RunData};

    // useful set of data for our unit tests
    const JSON: &'static str = r#"
       {
           "/usr/bin/zip": {
                "path": "/usr/bin/zip",
                "compressed": false, 
                "inode": 1,
                "dev": 1,
                "rundata": {
                    "tag1": {
                        "name": "tag1",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000
                    },
                    "tag2": {
                        "name": "tag2",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000
                    }
                }
            },
            "/etc/hosts.allow": {
                "path": "/etc/hosts.allow",
                "compressed": false, 
                "inode": 1,
                "dev": 1,
                "rundata": {
                    "tag3": {
                        "name": "tag3",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000
                    },
                    "tag4": {
                        "name": "tag4",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000
                    }
                }
            }
        }
    "#;

    fn load_json() -> HashMap<PathBuf, LogFile> {
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
        let mut json = crate::logfile::tests::load_json();

        assert!(json.contains_key(&PathBuf::from("/usr/bin/zip")));
        assert!(json.contains_key(&PathBuf::from("/etc/hosts.allow")));

        {
            let logfile1 = json.get_mut(&PathBuf::from("/usr/bin/zip")).unwrap();
            assert_eq!(logfile1.rundata.len(), 2);
            assert!(logfile1.contains_key("tag1"));
            assert!(logfile1.contains_key("tag2"));
        }

        {
            let logfile2 = json.get_mut(&PathBuf::from("/etc/hosts.allow")).unwrap();
            assert_eq!(logfile2.rundata.len(), 2);
            assert!(logfile2.contains_key("tag3"));
            assert!(logfile2.contains_key("tag4"));
        }
    }

    #[test]
    fn test_or_insert() {
        let mut json = crate::logfile::tests::load_json();

        {
            let rundata1 = json
                .get_mut(&PathBuf::from("/usr/bin/zip"))
                .unwrap()
                .or_insert("another_tag");

            assert_eq!(rundata1.name, "another_tag");
        }

        let logfile1 = json.get_mut(&PathBuf::from("/usr/bin/zip")).unwrap();
        let mut tags = logfile1.tags();
        tags.sort();
        assert_eq!(tags, vec!["another_tag", "tag1", "tag2"]);

        assert!(logfile1.contains_key("tag1"));
        assert!(!logfile1.contains_key("tag3"));

        // // tag4 is not part of LogFile
        // let mut rundata = json[1].or_insert("tag4");

        // // but tag3 is and even is duplicated
        // rundata = json[1].or_insert("tag3");
        // rundata.last_line = 999;

        // assert_eq!(json[1].rundata.get("tag3").unwrap().last_line, 999);
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