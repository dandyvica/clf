//! A structure representing a logfile, with all its related attributes. Those attributes are
//! coming from the processing of the log file, every time it's read to look for patterns.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use bzip2::read::BzDecoder;
use chrono::prelude::*;
use flate2::read::GzDecoder;
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize, Serializer};
use xz2::read::XzDecoder;

use crate::misc::{error::AppError, nagios::HitCounter};

use crate::config::{
    callback::ChildData,
    config::{GlobalOptions, Tag},
};

use crate::logfile::{
    compression::CompressionScheme,
    lookup::Lookup,
    lookup::ReaderCallType,
    signature::{FileIdentification, Signature},
};

/// Utility wrapper to pass all necessary references to the lookup methods.
#[derive(Debug)]
pub struct Wrapper<'a> {
    pub global: &'a GlobalOptions,
    pub tag: &'a Tag,
    pub logfile_counter: &'a mut HitCounter,
}

impl<'a> Wrapper<'a> {
    /// Simple helper for creating a new wrapper
    pub fn new(
        global: &'a GlobalOptions,
        tag: &'a Tag,
        logfile_counter: &'a mut HitCounter,
    ) -> Self {
        Wrapper {
            global,
            tag,
            logfile_counter,
        }
    }
}

/// A wrapper to store log file processing data.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct RunData {
    /// tag name
    //pub(super) tag_name: String,

    /// position of the last run. Used to seek the file pointer to this point.
    pub(super) last_offset: u64,

    /// last line number during the last search
    pub(super) last_line: u64,

    /// last time logfile were processed: printable date/time
    #[serde(serialize_with = "timestamp_to_string", skip_deserializing)]
    pub(super) last_run: f64,

    /// last time logfile were processed in seconds: used to check retention
    pub(super) last_run_secs: u64,

    /// ongoing critical count
    pub(super) critical_count: u64,

    /// ongoing warning count
    pub(super) warning_count: u64,

    /// number of script execution so far
    pub(super) exec_count: u16,
}

pub fn timestamp_to_string<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // exract integer part = number of seconds
    // frational part = number of nanoseconds
    let secs = value.trunc();
    let nanos = value.fract();
    let utc_tms = Utc.timestamp(secs as i64, (nanos * 1_000_000_000f64) as u32);
    format!("{}", utc_tms.format("%Y-%m-%d %H:%M:%S.%f")).serialize(serializer)
}

impl RunData {
    /// Returns the `tag_name` field value.
    // #[inline(always)]
    // pub fn get_tagname(&self) -> &str {
    //     &self.tag_name
    // }

    /// Returns the `last_run` field value.
    #[inline(always)]
    pub fn lastrun_secs(&self) -> u64 {
        self.last_run_secs
    }
}

/// A wrapper to get logfile information and its related attributes.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogFile {
    /// File & path as a `PathBuf`.
    pub(crate) path: PathBuf,

    /// Directory part or `None` if not existing.
    directory: Option<PathBuf>,

    /// Extension or `None` if no extension.
    extension: Option<String>,

    /// `true` if logfile is compressed.
    compression: CompressionScheme,

    /// Uniquely identifies a logfile
    signature: Signature,

    /// Run time data that are stored each time a logfile is searched for patterns.
    run_data: HashMap<String, RunData>,
}

impl LogFile {
    /// Creates a `LogFile` by providing the full logfile path. It also sets platform specific features
    /// like file *inode* or *dev*. The file path is checked for accessibility and is canonicalized. It also
    /// contains run time data, which correspond to the data created each time a logfile instance is searched
    /// for patterns.
    pub fn from_path<P: AsRef<Path>>(file_name: P) -> Result<LogFile, AppError> {
        // check if we can really use the file
        let path = PathBuf::from(file_name.as_ref());

        // logfiles should have an absolute path
        // if !path.is_absolute() {
        //     return Err(AppError::new(
        //         AppCustomErrorKind::FilePathNotAbsolute,
        //         "path {} is not absolute",
        //     ));
        // }

        let directory = path.parent().map(|p| p.to_path_buf());
        let extension = path.extension().map(|x| x.to_string_lossy().to_string());

        //const COMPRESSED_EXT: &[&str] = &["gz", "zip", "xz"];
        let compression = CompressionScheme::from(extension.as_deref());

        // canonicalize path: absolute form of the path with all intermediate
        // components normalized and symbolic links resolved.
        let canon = path.canonicalize()?;

        // // get inode & dev ID
        let signature = path.signature()?;

        Ok(LogFile {
            path: canon,
            directory,
            extension,
            compression,
            signature,
            run_data: HashMap::new(),
        })
    }

    // /// Insert rundata into the actual ones
    // pub fn insert_rundata(&mut self, other: &HashMap<String, RunData>) {
    //     self.run_data.into_iter().map(|(k, v)| other.insert(k, v));
    // }

    /// Return the path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Return the rundata
    pub fn rundata(&self) -> &HashMap<String, RunData> {
        &self.run_data
    }

    /// Set all rundata from another hashmap
    // pub fn set_rundata(&mut self, other_rundata: &HashMap<String, RunData>) {
    //     self.run_data = other_rundata.clone();
    // }

    /// Recalculate the signature to check whether it has changed
    pub fn has_changed(&self) -> Result<bool, AppError> {
        // get most recent signature
        let signature = self.path.signature()?;
        Ok(self.signature != signature && self.signature != Signature::default())
    }

    // /// Returns the list of tags of this `LogFile`.
    // #[cfg(test)]
    // pub fn tags(&self) -> Vec<&str> {
    //     self.run_data
    //         .keys()
    //         .map(|x| x.as_str())
    //         .collect::<Vec<&str>>()
    // }

    // /// Returns `true` if `name` is found in this `LogFile`.
    // #[cfg(test)]
    // pub fn contains_key(&self, name: &str) -> bool {
    //     self.run_data.contains_key(name)
    // }

    /// Returns an Option on a reference of a `RunData`, mapping the first
    /// tag name passed in argument.
    pub fn rundata_for_tag(&mut self, name: &str) -> &mut RunData {
        self.run_data.entry(name.to_string()).or_insert(RunData {
            //tag_name: name.to_string(),
            ..Default::default()
        })
    }

    /// Returns a reference on `Rundata`.
    #[inline(always)]
    pub fn rundata_mut(&mut self) -> &mut HashMap<String, RunData> {
        &mut self.run_data
    }

    /// Either delete \n or \r\n for end of line if line is ending by these
    pub fn purge_line(line: &mut Cow<str>) {
        if let Some(last_char) = line.chars().last() {
            if last_char == '\n' {
                line.to_mut().pop();
            }
        }
        #[cfg(target_family = "windows")]
        if let Some(last_char) = line.chars().last() {
            if last_char == '\r' {
                line.to_mut().pop();
            }
        }
    }

    //Just a wrapper function for a file.
    // pub fn lookup(
    //     &mut self,
    //     wrapper: &mut Wrapper,
    //     reader_type: &ReaderCallType,
    // ) -> Result<Vec<ChildData>, AppError> {
    //     // open target file
    //     let file = File::open(&self.path)?;

    //     // if file is compressed, we need to call a specific reader
    //     // create a specific reader for each compression scheme
    //     match self.compression {
    //         CompressionScheme::Gzip => {
    //             let decoder = GzDecoder::new(file);
    //             let reader = BufReader::new(decoder);
    //             //self.lookup_from_reader(reader, wrapper)
    //             reader_type.call(self, reader, wrapper)
    //         }
    //         CompressionScheme::Bzip2 => {
    //             let decoder = BzDecoder::new(file);
    //             let reader = BufReader::new(decoder);
    //             //self.lookup_from_reader(reader, wrapper)
    //             reader_type.call(self, reader, wrapper)
    //         }
    //         CompressionScheme::Xz => {
    //             let decoder = XzDecoder::new(file);
    //             let reader = BufReader::new(decoder);
    //             //self.lookup_from_reader(reader, wrapper)
    //             reader_type.call(self, reader, wrapper)
    //         }
    //         CompressionScheme::Uncompressed => {
    //             let reader = BufReader::new(file);
    //             //self.lookup_from_reader(reader, wrapper)
    //             reader_type.call(self, reader, wrapper)
    //         }
    //     }
    // }

    ///Just a wrapper function for a file.
    pub fn lookup<T>(
        &mut self,
        wrapper: &mut Wrapper,
        reader_type: &ReaderCallType,
    ) -> Result<Vec<ChildData>, AppError>
    where
        Self: Lookup<T>,
    {
        // open target file
        let file = File::open(&self.path)?;

        // if file is compressed, we need to call a specific reader
        // create a specific reader for each compression scheme
        match self.compression {
            CompressionScheme::Gzip => {
                let decoder = GzDecoder::new(file);
                let reader = BufReader::new(decoder);
                //self.lookup_from_reader(reader, wrapper)
                Lookup::<T>::reader(self, reader, wrapper)
            }
            CompressionScheme::Bzip2 => {
                let decoder = BzDecoder::new(file);
                let reader = BufReader::new(decoder);
                //self.lookup_from_reader(reader, wrapper)
                Lookup::<T>::reader(self, reader, wrapper)
            }
            CompressionScheme::Xz => {
                let decoder = XzDecoder::new(file);
                let reader = BufReader::new(decoder);
                //self.lookup_from_reader(reader, wrapper)
                Lookup::<T>::reader(self, reader, wrapper)
            }
            CompressionScheme::Uncompressed => {
                let reader = BufReader::new(file);
                //self.lookup_from_reader(reader, wrapper)
                Lookup::<T>::reader(self, reader, wrapper)
            }
        }
    }

    // Search for each tag in the search
    pub fn lookup_tags<T>(
        &mut self,
        global_opts: &GlobalOptions,
        tags: &[Tag],
        hit_counter: &mut HitCounter,
        reader_type: &ReaderCallType,
        children_list: &mut Vec<ChildData>,
    ) where
        Self: Lookup<T>,
    {
        for tag in tags.iter().filter(|t| t.process()) {
            debug!("searching for tag: {}", &tag.name());

            // wraps all structures into a helper struct
            let mut wrapper = Wrapper::new(global_opts, &tag, hit_counter);

            // now we can search for the pattern and save the child handle if a script was called
            match self.lookup::<T>(&mut wrapper, reader_type) {
                // script might be started, giving back a `Child` structure with process features like pid etc
                Ok(mut children) => {
                    // merge list of children
                    if children.len() != 0 {
                        children_list.append(&mut children);
                    }
                }

                // otherwise, an error when opening (most likely) the file and then report an error on counters
                Err(e) => {
                    error!(
                        "error: {} when searching logfile: {} for tag: {}",
                        e,
                        self.path.display(),
                        &tag.name()
                    );

                    // set error for this logfile
                    hit_counter.set_error(e);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    use crate::logfile::lookup::{FullReader, ReaderCallType};
    use crate::testing::setup::*;

    #[test]
    fn purge_line() {
        let s = "this an example\n";
        let mut cow: Cow<str> = Cow::Borrowed(s);
        LogFile::purge_line(&mut cow);
        assert_eq!(cow.into_owned(), "this an example");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn new() {
        let mut logfile = LogFile::from_path("/var/log/kern.log").unwrap();
        assert_eq!(logfile.path.to_str(), Some("/var/log/kern.log"));
        assert_eq!(logfile.directory.unwrap(), PathBuf::from("/var/log"));
        assert_eq!(logfile.extension.unwrap(), "log");
        assert_eq!(logfile.compression, CompressionScheme::Uncompressed);
        assert_eq!(logfile.run_data.len(), 0);

        logfile = LogFile::from_path("/etc/hosts").unwrap();
        assert_eq!(logfile.path.to_str(), Some("/etc/hosts"));
        assert!(logfile.extension.is_none());
        assert_eq!(logfile.compression, CompressionScheme::Uncompressed);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn new() {
        let mut logfile = LogFile::from_path(r"C:\Windows\System32\cmd.exe").unwrap();
        //assert_eq!(logfile.path.as_os_str(), std::ffi::OsStr::new(r"C:\Windows\System32\cmd.exe"));
        assert_eq!(logfile.extension.unwrap(), "exe");
        assert_eq!(
            logfile.directory.unwrap(),
            PathBuf::from(r"C:\Windows\System32")
        );
        assert_eq!(logfile.compression, CompressionScheme::Uncompressed);
        assert_eq!(logfile.run_data.len(), 0);

        logfile = LogFile::from_path(r"c:\windows\system32\drivers\etc\hosts").unwrap();
        assert!(logfile.extension.is_none());
    }
    //#[test]
    fn from_reader() {
        let opts = GlobalOptions::from_str("path: /usr/bin").expect("unable to read YAML");

        let yaml = r#"
            name: test
            options: "runcallback"
            process: true
            callback: { 
                address: "127.0.0.1:8999",
                args: ['arg1', 'arg2', 'arg3']
            }
            patterns:
                critical: {
                    regexes: [
                        '^ERROR: opening file "([a-z0-9/]*)" from node ([\w\.]+), error = (\d)',
                    ],
                    exceptions: [
                        'error = 5'
                    ]
                }
                warning: {
                    regexes: [
                        '^WARNING: opening file "([a-z0-9/]*)" from node ([\w\.]+), error = (\d)',
                    ],
                    exceptions: [
                        'error = 5'
                    ]
                }
        "#;

        let tag = Tag::from_str(yaml).expect("unable to read YAML");
        let mut logfile_counter = HitCounter::default();

        let mut w = Wrapper::new(&opts, &tag, &mut logfile_counter);

        let mut logfile = LogFile::from_path("tests/logfiles/adhoc.txt").unwrap();

        // create a very simple TCP server: wait for data and test them
        let child = std::thread::spawn(move || {
            // create a listener
            let listener = std::net::TcpListener::bind("127.0.0.1:8999").unwrap();
            match listener.accept() {
                Ok((mut socket, _addr)) => loop {
                    let json = get_json_from_stream(&mut socket);

                    if json.is_err() {
                        break;
                    }

                    let json_data = json.unwrap();
                    //dbg!(&json_data);

                    match json_data.vars.get("CLF_MATCHED_RE_TYPE").unwrap().as_str() {
                        "critical" => {
                            assert_eq!(json_data.args, vec!["arg1", "arg2", "arg3"]);
                            assert_eq!(
                                json_data.vars.get("CLF_CAPTURE1").unwrap(),
                                &format!(
                                    "{}{}",
                                    "/var/log/messages",
                                    json_data.vars.get("CLF_LINE_NUMBER").unwrap()
                                )
                            );
                            assert_eq!(
                                json_data.vars.get("CLF_CAPTURE2").unwrap(),
                                "server01.domain.com"
                            );
                            assert_ne!(json_data.vars.get("CLF_CAPTURE3").unwrap(), "5");
                        }
                        "warning" => {
                            assert_eq!(json_data.args, vec!["arg1", "arg2", "arg3"]);
                            assert_eq!(
                                json_data.vars.get("CLF_CAPTURE1").unwrap(),
                                &format!(
                                    "{}{}",
                                    "/var/log/syslog",
                                    json_data.vars.get("CLF_LINE_NUMBER").unwrap()
                                )
                            );
                            assert_eq!(
                                json_data.vars.get("CLF_CAPTURE2").unwrap(),
                                "server01.domain.com"
                            );
                            assert_ne!(json_data.vars.get("CLF_CAPTURE3").unwrap(), "5");
                        }
                        "ok" => (),
                        &_ => panic!("unexpected case"),
                    }
                },
                Err(e) => panic!("couldn't get client: {:?}", e),
            }
        });

        // wait a little
        let ten_millis = std::time::Duration::from_millis(10);
        std::thread::sleep(ten_millis);

        let _ret = logfile.lookup::<FullReader>(&mut w, &ReaderCallType::FullReaderCall);
        let _res = child.join();
    }
}
