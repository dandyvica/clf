//! A structure representing a logfile, with all its related attributes. Those attributes are
//! coming from the processing of the log file, every time it's read to look for patterns.
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use xz2::read::XzDecoder;

use crate::config::{
    callback::ChildData, global::GlobalOptions, logfiledef::LogFileDef, pattern::PatternCounters,
    tag::Tag,
};
use crate::context;
use crate::logfile::{
    compression::CompressionScheme, logfileid::LogFileID, lookup::Lookup, rundata::RunData,
};
use crate::misc::error::{AppError, AppResult};
use crate::misc::extension::{ReadFs, Signature};

/// A wrapper to get logfile information and its related attributes.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LogFile {
    /// All fields depending on the declared path
    pub id: LogFileID,

    /// All other fields from the config file
    #[serde(skip)]
    pub definition: LogFileDef,

    /// Run time data that are stored each time a logfile is searched for patterns.
    pub run_data: HashMap<String, RunData>,
}

impl LogFile {
    /// Creates a `LogFile` by providing the full logfile path. It also sets platform specific features
    /// like file *inode* or *dev*. The file path is checked for accessibility and is canonicalized. It also
    /// contains run time data, which correspond to the data created each time a logfile instance is searched
    /// for patterns.
    pub fn from_path<P: AsRef<Path>>(path: P) -> AppResult<LogFile> {
        // create a default logfile and update it later. This is used to not duplicate code
        let mut logfile = LogFile::default();

        logfile.id.update(path)?;

        Ok(logfile)
    }

    /// Set definition coming from config file
    pub fn set_definition(&mut self, def: LogFileDef) {
        self.definition = def;
    }

    /// Recalculate the signature to check whether it has changed
    pub fn has_changed(&self) -> AppResult<bool> {
        // get most recent signature
        let signature = self.id.canon_path.signature()?;
        trace!(
            "file = {:?}, current signature = {:?}, recalculated = {:?}",
            &self.id.canon_path,
            self.id.signature,
            signature
        );
        Ok(self.id.signature != signature && self.id.signature != Signature::default())
    }

    /// Returns an Option on a reference of a `RunData`, mapping the first
    /// tag name passed in argument.
    pub fn rundata_for_tag(&mut self, name: &str) -> &mut RunData {
        self.run_data
            .entry(name.to_string())
            .or_insert(RunData::default())
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

    /// Sum all counters from `rundata` for all tags
    pub fn sum_counters(&self) -> PatternCounters {
        self.run_data.values().map(|x| &x.counters).sum()
    }

    /// Last error occuring when reading this logfile
    pub fn set_error(&mut self, error: AppError, tag_name: &str) {
        debug_assert!(self.run_data.contains_key(tag_name));
        self.run_data.get_mut(tag_name).unwrap().last_error = Some(error);
    }

    ///Just a wrapper function for a file.
    pub fn lookup<T>(
        &mut self,
        tag: &Tag,
        global_options: &GlobalOptions,
    ) -> AppResult<Vec<ChildData>>
    where
        Self: Lookup<T>,
    {
        // open target file
        let file = File::open(&self.id.canon_path)
            .map_err(|e| context!(e, "unable to open file:{:?}", &self.id.canon_path))?;

        // if file is compressed, we need to call a specific reader
        // create a specific reader for each compression scheme
        match self.id.compression {
            CompressionScheme::Gzip => {
                let decoder = GzDecoder::new(file);
                let reader = BufReader::new(decoder);
                //self.lookup_from_reader(reader, wrapper)
                Lookup::<T>::reader(self, reader, tag, global_options)
            }
            CompressionScheme::Bzip2 => {
                let decoder = BzDecoder::new(file);
                let reader = BufReader::new(decoder);
                //self.lookup_from_reader(reader, wrapper)
                Lookup::<T>::reader(self, reader, tag, global_options)
            }
            CompressionScheme::Xz => {
                let decoder = XzDecoder::new(file);
                let reader = BufReader::new(decoder);
                //self.lookup_from_reader(reader, wrapper)
                Lookup::<T>::reader(self, reader, tag, global_options)
            }
            CompressionScheme::Uncompressed => {
                let reader = BufReader::new(file);
                //self.lookup_from_reader(reader, wrapper)
                Lookup::<T>::reader(self, reader, tag, global_options)
            }
        }
    }

    // Search for each tag in the search
    pub fn lookup_tags<T>(
        &mut self,
        global_options: &GlobalOptions,
        tags: &[Tag],
        children_list: &mut Vec<ChildData>,
    ) where
        Self: Lookup<T>,
    {
        for tag in tags.iter().filter(|t| t.process) {
            debug!("searching for tag: {}", &tag.name);

            // now we can search for the pattern and save the child handle if a script was called
            match self.lookup::<T>(tag, global_options) {
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
                        self.id.canon_path.display(),
                        &tag.name
                    );

                    // set error for this logfile
                    self.set_error(e, &tag.name);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    use crate::config::vars::Vars;
    #[derive(Debug, Deserialize)]
    struct JSONStream {
        pub args: Vec<String>,
        pub vars: Vars<String, String>,
    }

    // utility fn to receive JSON from a stream
    // fn get_json_from_stream<T: std::io::Read>(
    //     socket: &mut T,
    // ) -> Result<JSONStream, std::io::Error> {
    //     // try to read size first
    //     let mut size_buffer = [0; std::mem::size_of::<u16>()];
    //     let bytes_read = socket.read(&mut size_buffer)?;
    //     //dbg!(bytes_read);
    //     if bytes_read == 0 {
    //         return Err(Error::new(ErrorKind::Interrupted, "socket closed"));
    //     }

    //     let json_size = u16::from_be_bytes(size_buffer);

    //     // read JSON raw data
    //     let mut json_buffer = vec![0; json_size as usize];
    //     socket.read_exact(&mut json_buffer).unwrap();

    //     // get JSON
    //     let s = std::str::from_utf8(&json_buffer).unwrap();

    //     let json: JSONStream = serde_json::from_str(&s).unwrap();
    //     Ok(json)
    // }

    #[test]
    #[cfg(target_family = "unix")]
    fn purge_line() {
        let s = "this an example\n";
        let mut cow: Cow<str> = Cow::Borrowed(s);
        LogFile::purge_line(&mut cow);
        assert_eq!(cow.into_owned(), "this an example");
    }

    #[test]
    #[cfg(target_family = "windows")]
    fn purge_line() {
        let s = "this an example\r\n";
        let mut cow: Cow<str> = Cow::Borrowed(s);
        LogFile::purge_line(&mut cow);
        assert_eq!(cow.into_owned(), "this an example");
    }

    //#[test]
    #[cfg(target_os = "linux")]
    fn new() {
        let mut logfile = LogFile::from_path("/var/log/kern.log").unwrap();
        assert_eq!(logfile.id.declared_path.to_str(), Some("/var/log/kern.log"));
        assert_eq!(logfile.id.canon_path.to_str(), Some("/var/log/kern.log"));
        assert_eq!(
            logfile.id.directory.unwrap(),
            std::path::PathBuf::from("/var/log")
        );
        assert_eq!(logfile.id.extension.unwrap(), "log");
        assert_eq!(logfile.id.compression, CompressionScheme::Uncompressed);
        assert_eq!(logfile.run_data.len(), 0);

        logfile = LogFile::from_path("/etc/hosts").unwrap();
        assert_eq!(logfile.id.canon_path.to_str(), Some("/etc/hosts"));
        assert!(logfile.id.extension.is_none());
        assert_eq!(logfile.id.compression, CompressionScheme::Uncompressed);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn new() {
        let mut logfile = LogFile::from_path(r"C:\Windows\System32\cmd.exe").unwrap();
        //assert_eq!(logfile.path.as_os_str(), std::ffi::OsStr::new(r"C:\Windows\System32\cmd.exe"));
        assert_eq!(logfile.id.extension.unwrap(), "exe");
        assert_eq!(
            logfile.id.directory.unwrap(),
            PathBuf::from(r"\\?\C:\Windows\System32")
        );
        assert_eq!(logfile.id.compression, CompressionScheme::Uncompressed);
        assert_eq!(logfile.run_data.len(), 0);

        logfile = LogFile::from_path(r"c:\windows\system32\drivers\etc\hosts").unwrap();
        assert!(logfile.id.extension.is_none());
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn from_reader() {
        let global = GlobalOptions::from_str("path: /usr/bin").expect("unable to read YAML");

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

        let mut tag = Tag::from_str(yaml).expect("unable to read YAML");

        let mut logfile = LogFile::from_path("tests/unittest/adhoc.txt").unwrap();

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

        let _ret = logfile.lookup::<FullReader>(&mut tag, &global);
        let _res = child.join();
    }
}
