//! A structure representing a logfile, with all its related attributes. Those attributes are
//! coming from the processing of the log file, every time it's read to look for patterns.
use log::{debug, info};
use std::collections::HashMap;
use std::fs::{File, Metadata};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};

use crate::{
    callback::ChildReturn,
    config::{GlobalOptions, Tag},
    error::{AppCustomErrorKind, AppError},
    pattern::PatternType,
    variables::Variables,
};

/// Creates new buffer with this initial capacity.
const BUFFER_SIZE: usize = 1024;

/// A wrapper to store log file processing data.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RunData {
    /// tag name
    #[serde(rename = "name")]
    tag_name: String,

    /// position of the last run. Used to seek the file pointer to this point.
    last_offset: u64,

    /// last line number during the last search
    last_line: u64,

    /// last time logfile is processed
    last_run: u64,
}

impl RunData {
    /// Returns the `tag_name` field value.
    #[inline(always)]
    pub fn get_tagname(&self) -> &str {
        &self.tag_name
    }

    /// Returns the `last_run` field value.
    #[inline(always)]
    pub fn get_lastrun(&self) -> u64 {
        self.last_run
    }
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
        let directory = path.parent().map(|p| p.to_path_buf());
        let extension = path.extension().map(|x| x.to_string_lossy().to_string());

        // if !path.is_usable() {
        //     return Err(AppError::App {
        //         err: AppCustomErrorKind::FileNotUsable,
        //         msg: format!("file {:?} is not usable", path),
        //     });
        // }

        const COMPRESSED_EXT: &[&str] = &["gz", "zip", "xz"];
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
        let ids = LogFile::get_ids(&metadata);

        Ok(LogFile {
            path: canon,
            directory,
            extension,
            compressed,
            inode: ids.0,
            dev: ids.1,
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
            tag_name: name.to_string(),
            ..Default::default()
        })
    }

    /// Returns a reference on `Rundata`.
    #[inline(always)]
    pub fn get_mut_rundata(&mut self) -> &mut HashMap<String, RunData> {
        &mut self.rundata
    }

    /// Sets UNIX inode and dev identifiers.
    #[cfg(target_family = "unix")]
    pub fn get_ids(metadata: &Metadata) -> (u64, u64) {
        (metadata.ino(), metadata.dev())
    }

    #[cfg(target_family = "windows")]
    pub fn get_ids(metadata: &Metadata) -> (u64, u64) {
        (0, 0)
    }
}

/// Two log files are considered equal if they have the same name, inode & dev.
impl PartialEq for LogFile {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.dev == other.dev && self.inode == other.inode
    }
}

/// Trait which provides a seek function, and is implemented for all
/// `BufReader<T>` types used in `Lookup` trait.
pub trait Seeker {
    /// Simulates the `seek`method for all used `BufReader<R>`.
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError>;
}

impl Seeker for BufReader<File> {
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
        self.seek(SeekFrom::Start(offset)).map_err(AppError::Io)
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

/// Utility wrapper to pass all necessrary reference to the lookup methods.
pub struct Wrapper<'a> {
    pub global: &'a GlobalOptions,
    pub tag: &'a Tag,
    pub vars: &'a mut Variables,
}

/// Return type for all `Lookup` methods.
pub type LookupReturn<T> = Result<T, AppError>;
pub type LookupRet = LookupReturn<Option<ChildReturn>>;

/// Trait, implemented by `LogFile` to search patterns.
pub trait Lookup {
    fn lookup(&mut self, wrapper: &mut Wrapper) -> LookupRet;
    fn lookup_from_reader<R: BufRead + Seeker>(
        &mut self,
        reader: R,
        wrapper: &mut Wrapper,
    ) -> LookupRet;
}

impl Lookup for LogFile {
    ///Just a wrapper function for a file.
    fn lookup(&mut self, wrapper: &mut Wrapper) -> LookupRet {
        // open target file
        let file = File::open(&self.path)?;

        // if file is compressed, we need to call a specific reader
        match self.extension.as_deref() {
            Some("gz") => {
                let decoder = GzDecoder::new(file);
                let reader = BufReader::new(decoder);
                self.lookup_from_reader(reader, wrapper)
            }
            Some(&_) | None => {
                let reader = BufReader::new(file);
                self.lookup_from_reader(reader, wrapper)
            }
        }
    }

    fn lookup_from_reader<R: BufRead + Seeker>(
        &mut self,
        mut reader: R,
        wrapper: &mut Wrapper,
    ) -> LookupRet {
        // uses the same buffer
        let mut buffer = Vec::with_capacity(BUFFER_SIZE);

        // defined a new child handle
        let mut child_return: Option<ChildReturn> = None;

        // this will count number of matches for warning & critical, to see if this matches the thresholds
        // first is warning, second is critical
        let mut thresholds = (0u16, 0u16);

        // anyway, reset only runtime variables
        wrapper.vars.runtime_vars.clear();
        wrapper.vars.insert(
            "LOGFILE",
            self.path.to_str().unwrap_or("error converting PathBuf"),
        );

        // get rundata corresponding to tag name, or insert that new one is not yet in snapshot
        let mut rundata = self.or_insert(&wrapper.tag.name);
        println!(
            "tagname: {:?}, rundata:{:?}\n\n",
            &wrapper.tag.name, rundata
        );

        // initialize counters
        let mut bytes_count = 0;
        let mut line_number = 0;

        // if we don't need to read the file from the beginning, adjust counters and set offset
        if !wrapper.tag.options.rewind {
            bytes_count = rundata.last_offset;
            line_number = rundata.last_line;
            reader.set_offset(rundata.last_offset)?;
        }

        // move to position if already recorded, and not rewind
        //if !tag.options.rewind && rundata.last_offset != 0 {
        // if !tag.options.rewind && rundata.last_offset != 0 {
        //     reader.set_offset(rundata.last_offset)?;
        // }

        info!(
            "starting read from last offset={}, last line={}",
            bytes_count, line_number
        );

        loop {
            // read until \n (which is included in the buffer)
            let ret = reader.read_until(b'\n', &mut buffer);

            // to deal with UTF-8 conversion problems, use the lossy method. It will replace non-UTF-8 chars with ?
            let line = String::from_utf8_lossy(&buffer);

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

                    // is there a match, regarding also exceptions?
                    if let Some(re) = wrapper.tag.is_match(&line) {
                        // increments thresholds and compare with possible defined limits
                        match re.0 {
                            PatternType::warning => {
                                thresholds.0 += 1;
                                if thresholds.0 < wrapper.tag.options.warningthreshold {
                                    buffer.clear();
                                    continue;
                                }
                            }
                            PatternType::critical => {
                                thresholds.1 += 1;
                                if thresholds.1 < wrapper.tag.options.criticalthreshold {
                                    buffer.clear();
                                    continue;
                                }
                            }
                            _ => (),
                        };

                        debug!(
                            "found a match tag={}, line={}, line#={}, re=({:?},{}), thresholds={:?}",
                            wrapper.tag.name,
                            line.clone(),
                            line_number,
                            re.0,
                            re.1.as_str(),
                            thresholds
                        );

                        // create variables which will be set as environment variables when script is called
                        wrapper
                            .vars
                            .insert("LINE_NUMBER", format!("{}", line_number));
                        wrapper.vars.insert("LINE", line.clone());
                        wrapper.vars.insert("MATCHED_RE", re.1.as_str());
                        wrapper.vars.insert("MATCHED_RE_TYPE", re.0);
                        wrapper.vars.insert("TAG", &wrapper.tag.name);

                        wrapper.vars.insert_captures(re.1, &line);

                        debug!("added variables: {:?}", wrapper.vars);

                        // now call script if there's an external script to call, if we're told to do so
                        //debug_assert!(&tag.options.is_some());
                        if wrapper.tag.options.runscript {
                            child_return = wrapper.tag.call_script(None, wrapper.vars)?;
                        };
                    }

                    // reset buffer to not accumulate data
                    buffer.clear();
                }
                // a rare IO error could occur here
                Err(err) => {
                    debug!("read_line() error kind: {:?}, line: {}", err.kind(), line);
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

        Ok(child_return)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::*;
    use crate::error::*;

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
    fn new() {
        let mut lf_ok = LogFile::new("/usr/bin/zip").unwrap();
        assert_eq!(lf_ok.path.to_str(), Some("/usr/bin/zip"));
        assert!(lf_ok.extension.is_none());

        lf_ok = LogFile::new("/etc/hosts.allow").unwrap();
        assert_eq!(lf_ok.path.to_str(), Some("/etc/hosts.allow"));
        assert_eq!(lf_ok.extension.unwrap(), "allow");

        // // file not found
        // let mut lf_err = LogFile::new("/usr/bin/foo");
        // assert!(lf_err.is_err());
        // match lf_err.unwrap_err() {
        //     AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomErrorKind::FileNotUsable),
        //     _ => panic!("error not expected here!"),
        // };

        // // not a file
        // lf_err = LogFile::new("/usr/bin");
        // match lf_err.unwrap_err() {
        //     AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomErrorKind::FileNotUsable),
        //     _ => panic!("error not expected here!"),
        // };

        // // file has no root
        // lf_err = LogFile::new("usr/bin/foo");
        // assert!(lf_err.is_err());
        // match lf_err.unwrap_err() {
        //     AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomErrorKind::FileNotUsable),
        //     _ => panic!("error not expected here!"),
        // };
    }

    #[test]
    fn deserialize() {
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
    fn or_insert() {
        let mut json = crate::logfile::tests::load_json();

        {
            let rundata1 = json
                .get_mut(&PathBuf::from("/usr/bin/zip"))
                .unwrap()
                .or_insert("another_tag");

            assert_eq!(rundata1.tag_name, "another_tag");
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

    #[test]
    #[cfg(target_os = "windows")]
    fn new() {
        let mut lf_ok = LogFile::new(r"C:\Windows\System32\cmd.exe").unwrap();
        //assert_eq!(lf_ok.path.as_os_str(), std::ffi::OsStr::new(r"C:\Windows\System32\cmd.exe"));
        assert_eq!(lf_ok.extension.unwrap(), "exe");

        lf_ok = LogFile::new(r"c:\windows\system32\drivers\etc\hosts").unwrap();
        //assert_eq!(lf_ok.path.as_os_str(), std::ffi::OsStr::new(r"C:\Windows\System32\cmd.exe"));
        assert!(lf_ok.extension.is_none());

        // // file not found
        // let mut lf_err = LogFile::new(r"C:\Windows\System32\foo.exe");
        // assert!(lf_err.is_err());
        // match lf_err.unwrap_err() {
        //     AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomErrorKind::FileNotUsable),
        //     _ => panic!("error not expected here!"),
        // };

        // // not a file
        // lf_err = LogFile::new(r"C:\Windows\System32");
        // match lf_err.unwrap_err() {
        //     AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomErrorKind::FileNotUsable),
        //     _ => panic!("error not expected here!"),
        // };

        // // file has no root
        // lf_err = LogFile::new(r"Windows\System32\cmd.exe");
        // assert!(lf_err.is_err());
        // match lf_err.unwrap_err() {
        //     AppError::App { err: e, msg: _ } => assert_eq!(e, AppCustomErrorKind::FileNotUsable),
        //     _ => panic!("error not expected here!"),
        // };
    }
}
