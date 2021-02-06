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

use crate::configuration::{
    callback::ChildData, global::GlobalOptions, logfiledef::LogFileDef, pattern::PatternCounters,
    tag::Tag,
};
use crate::context;
use crate::logfile::{
    compression::CompressionScheme, logfileid::LogFileID, lookup::Lookup, rundata::RunData,
};
use crate::misc::error::{AppCustomErrorKind, AppError, AppResult};
use crate::misc::extension::ReadFs;

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
    /// for patterns. If a definition is provided, assign it
    pub fn from_path<P: AsRef<Path>>(path: P, def: Option<LogFileDef>) -> AppResult<LogFile> {
        // create a default logfile and update it later. This is used to not duplicate code
        let mut logfile = LogFile::default();

        // if a definition is provided, assign it
        if let Some(definition) = def {
            logfile.definition = definition;
        }

        // now update all fields
        logfile.id.update(path, logfile.definition.hash_window)?;

        Ok(logfile)
    }

    /// Set definition coming from config file
    pub fn set_definition(&mut self, def: LogFileDef) {
        self.definition = def;
    }

    /// Recalculate the signature to check whether it has changed
    pub fn hash_been_rotated(&self) -> AppResult<bool> {
        // get most recent signature
        let old_signature = &self.id.signature;
        let new_signature = self.id.canon_path.signature(self.definition.hash_window)?;

        trace!(
            "file = {:?}, current signature = {:?}, recalculated = {:?}",
            &self.id.canon_path,
            old_signature,
            new_signature
        );

        // dev number are different: files are located in different file systems
        if old_signature.dev != new_signature.dev {
            Ok(true)
        }
        // dev are equal but inodes are different
        else if old_signature.inode != new_signature.inode {
            Ok(true)
        }
        // dev, inodes are equal => test hashes
        else {
            // if either hash is None (this means the file size is < hash_window) we can't decide
            if old_signature.hash.is_none() || new_signature.hash.is_none() {
                Err(AppError::new_custom(
                    AppCustomErrorKind::FileSizeIsLessThanHashWindow,
                    &format!(
                        "unable to determine a safe hash for logfile {:?}",
                        self.id.declared_path
                    ),
                ))
            }
            // if hashes are equal we can assume file has not been rotated
            else if old_signature.hash.unwrap() == new_signature.hash.unwrap() {
                Ok(false)
            }
            // if not we can assume this is a new file
            else {
                Ok(true)
            }
        }
    }

    // pub fn get_signatures(&self) -> (Signature, Signature) {
    //     let new_signature = self.id.canon_path.signature().unwrap();
    //     (self.id.signature.clone(), new_signature)
    // }

    /// Returns an Option on a reference of a `RunData`, mapping the first
    /// tag name passed in argument.
    pub fn rundata_for_tag(&mut self, name: &str) -> &mut RunData {
        self.run_data
            .entry(name.to_string())
            .or_insert_with(RunData::default)
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

    /// Sum all counters from `rundata` for all tags, but excluding the value of id
    pub fn sum_counters(&self, id: u32) -> PatternCounters {
        self.run_data
            .values()
            .filter(|x| x.pid == id)
            .map(|x| &x.counters)
            .sum()
    }

    /// Last error occuring when reading this logfile
    pub fn set_error(&mut self, error: AppError, tag_name: &str) {
        debug_assert!(self.run_data.contains_key(tag_name));
        self.run_data.get_mut(tag_name).unwrap().last_error = Some(error);
    }

    /// Reset counters and offsets for a specific tag
    pub fn reset_tag(&mut self, tag_name: &str) {
        debug_assert!(self.run_data.contains_key(tag_name));

        let tag = self.run_data.get_mut(tag_name).unwrap();
        tag.counters = PatternCounters::default();
        tag.last_line = 0;
        tag.last_offset = 0;
    }

    /// Reset offsets for a specific tag
    pub fn reset_tag_offsets(&mut self, tag_name: &str) {
        debug_assert!(self.run_data.contains_key(tag_name));

        let tag = self.run_data.get_mut(tag_name).unwrap();
        tag.last_line = 0;
        tag.last_offset = 0;
    }

    /// Copy counters from another logfile
    pub fn copy_counters(&mut self, other: &Self, tag_name: &str) {
        debug_assert!(self.run_data.contains_key(tag_name));
        debug_assert!(other.run_data.contains_key(tag_name));

        let tag = self.run_data.get_mut(tag_name).unwrap();
        tag.counters = other.run_data.get(tag_name).unwrap().counters.clone();
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
                    if !children.is_empty() {
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
    use std::str::FromStr;

    use super::*;

    use crate::configuration::vars::VarType;

    #[derive(Debug, Deserialize)]
    struct JSONStream {
        pub args: Vec<String>,
        pub vars: std::collections::HashMap<String, VarType<String>>,
    }

    // utility fn to receive JSON from a stream
    #[cfg(target_family = "unix")]
    fn get_json_from_stream<T: std::io::Read>(
        socket: &mut T,
    ) -> Result<JSONStream, std::io::Error> {
        use std::io::{Error, ErrorKind};

        // try to read size first
        let mut size_buffer = [0; std::mem::size_of::<u16>()];
        let bytes_read = socket.read(&mut size_buffer)?;
        //dbg!(bytes_read);
        if bytes_read == 0 {
            return Err(Error::new(ErrorKind::Interrupted, "socket closed"));
        }

        let json_size = u16::from_be_bytes(size_buffer);

        // read JSON raw data
        let mut json_buffer = vec![0; json_size as usize];
        socket.read_exact(&mut json_buffer).unwrap();

        // get JSON
        let s = std::str::from_utf8(&json_buffer).unwrap();

        let json: JSONStream = serde_json::from_str(&s).unwrap();
        Ok(json)
    }

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

    #[test]
    #[cfg(target_os = "linux")]
    fn new() {
        let mut def = LogFileDef::default();
        def.hash_window = 4096;

        let mut logfile = LogFile::from_path("./tests/unittest/list_files.log", Some(def.clone())).unwrap();
        assert_eq!(logfile.id.declared_path.to_str(), Some("./tests/unittest/list_files.log"));
        assert!(logfile.id.canon_path.to_str().unwrap().contains("list_files"));
        // assert_eq!(
        //     logfile.id.directory.unwrap(),
        //     std::path::PathBuf::from("/var/log")
        // );
        assert_eq!(logfile.id.extension.unwrap(), "log");
        assert_eq!(logfile.id.compression, CompressionScheme::Uncompressed);
        assert_eq!(logfile.run_data.len(), 0);

        logfile = LogFile::from_path("/etc/hosts", Some(def.clone())).unwrap();
        assert_eq!(logfile.id.canon_path.to_str(), Some("/etc/hosts"));
        assert!(logfile.id.extension.is_none());
        assert_eq!(logfile.id.compression, CompressionScheme::Uncompressed);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn new() {
        let mut def = LogFileDef::default();
        def.hash_window = 4096;

        let mut logfile =
            LogFile::from_path(r"C:\Windows\System32\cmd.exe", Some(def.clone())).unwrap();
        //assert_eq!(logfile.path.as_os_str(), std::ffi::OsStr::new(r"C:\Windows\System32\cmd.exe"));
        assert_eq!(logfile.id.extension.unwrap(), "exe");
        assert_eq!(
            logfile.id.directory.unwrap(),
            std::path::PathBuf::from(r"\\?\C:\Windows\System32")
        );
        assert_eq!(logfile.id.compression, CompressionScheme::Uncompressed);
        assert_eq!(logfile.run_data.len(), 0);

        logfile = LogFile::from_path(r"c:\windows\system32\drivers\etc\hosts", Some(def.clone()))
            .unwrap();
        assert!(logfile.id.extension.is_none());
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn from_reader() {
        let global = GlobalOptions::from_str("script_path: /usr/bin").expect("unable to read YAML");

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

        let mut def = LogFileDef::default();
        def.hash_window = 4096;

        let mut logfile = LogFile::from_path("tests/unittest/adhoc.txt", Some(def)).unwrap();

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

                    match json_data
                        .vars
                        .get("CLF_MATCHED_RE_TYPE")
                        .unwrap()
                        .as_t()
                        .as_str()
                    {
                        "critical" => {
                            assert_eq!(json_data.args, vec!["arg1", "arg2", "arg3"]);
                            assert_eq!(
                                json_data.vars.get("CLF_CG_2").unwrap(),
                                &VarType::from("server01.domain.com")
                            );
                            assert_ne!(
                                json_data.vars.get("CLF_CG_3").unwrap(),
                                &VarType::from("5")
                            );
                        }
                        "warning" => {
                            assert_eq!(json_data.args, vec!["arg1", "arg2", "arg3"]);
                            // assert_eq!(
                            //     json_data.vars.get("CLF_CG_1").unwrap(),
                            //     &format!(
                            //         "{}{}",
                            //         "/var/log/syslog",
                            //         json_data.vars.get("CLF_LINE_NUMBER").unwrap()
                            //     )
                            // );
                            assert_eq!(
                                json_data.vars.get("CLF_CG_2").unwrap(),
                                &VarType::from("server01.domain.com")
                            );
                            assert_ne!(
                                json_data.vars.get("CLF_CG_3").unwrap(),
                                &VarType::from("5")
                            );
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

        let _ret = logfile.lookup::<crate::logfile::lookup::FullReader>(&mut tag, &global);
        let _res = child.join();
    }
}
