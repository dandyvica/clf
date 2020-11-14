//! A structure representing a logfile, with all its related attributes. Those attributes are
//! coming from the processing of the log file, every time it's read to look for patterns.
use log::{debug, error, info, trace};
use std::collections::HashMap;
use std::fs::{File, Metadata};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};

use crate::misc::{
    error::{AppCustomErrorKind, AppError},
    nagios::HitCounter,
    util::Cons,
};

use crate::config::{
    callback::{CallbackHandle, ChildData},
    config::{GlobalOptions, Tag},
    pattern::PatternType,
    variables::Variables,
};

use crate::var;

/// A wrapper to store log file processing data.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RunData {
    /// tag name
    tag_name: String,

    /// position of the last run. Used to seek the file pointer to this point.
    last_offset: u64,

    /// last line number during the last search
    last_line: u64,

    /// last time logfile is processed
    last_run: u64,

    /// ongoing critical count
    critical_count: u64,

    /// ongoing warning count
    warning_count: u64,

    /// number of script execution so far
    exec_count: u16,
}

impl RunData {
    /// Returns the `tag_name` field value.
    // #[inline(always)]
    // pub fn get_tagname(&self) -> &str {
    //     &self.tag_name
    // }

    /// Returns the `last_run` field value.
    #[inline(always)]
    pub fn lastrun(&self) -> u64 {
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
    run_data: HashMap<String, RunData>,
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

        //const COMPRESSED_EXT: &[&str] = &["gz", "zip", "xz"];
        let compressed = match &extension {
            None => false,
            Some(ext) => ext == "gz",
        };

        // canonicalize path: absolute form of the path with all intermediate
        // components normalized and symbolic links resolved.
        let canon = path.canonicalize()?;

        // get metadata if possible
        let metadata = path.metadata()?;

        // get inode & dev ID
        let ids = LogFile::ids(&metadata);

        Ok(LogFile {
            path: canon,
            directory,
            extension,
            compressed,
            inode: ids.0,
            dev: ids.1,
            run_data: HashMap::new(),
        })
    }

    /// Returns the list of tags of this `LogFile`.
    #[cfg(test)]
    pub fn tags(&self) -> Vec<&str> {
        self.run_data
            .keys()
            .map(|x| x.as_str())
            .collect::<Vec<&str>>()
    }

    /// Returns `true` if `name` is found in this `LogFile`.
    #[cfg(test)]
    pub fn contains_key(&self, name: &str) -> bool {
        self.run_data.contains_key(name)
    }

    /// Returns an Option on a reference of a `RunData`, mapping the first
    /// tag name passed in argument.
    pub fn rundata(&mut self, name: &str) -> &mut RunData {
        self.run_data.entry(name.to_string()).or_insert(RunData {
            tag_name: name.to_string(),
            ..Default::default()
        })
    }

    /// Returns a reference on `Rundata`.
    #[inline(always)]
    pub fn rundata_mut(&mut self) -> &mut HashMap<String, RunData> {
        &mut self.run_data
    }

    /// Sets UNIX inode and dev identifiers.
    #[cfg(target_family = "unix")]
    pub fn ids(metadata: &Metadata) -> (u64, u64) {
        (metadata.ino(), metadata.dev())
    }

    #[cfg(target_family = "windows")]
    pub fn ids(metadata: &Metadata) -> (u64, u64) {
        (0, 0)
    }

    /// True is file is compressed.
    #[inline(always)]
    fn is_gzipped(&self) -> bool {
        self.compressed
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
    #[inline(always)]
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
        self.seek(SeekFrom::Start(offset)).map_err(AppError::Io)
    }
}

/// Implementing for T: Read helps testing wuth `Cursor` type.
impl<T> Seeker for BufReader<GzDecoder<T>>
where
    T: Read,
{
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
        // if 0, nothing to do
        if offset == 0 {
            return Ok(0);
        }

        let pos = match self.by_ref().bytes().nth((offset - 1) as usize) {
            None => {
                return Err(AppError::new(
                    AppCustomErrorKind::SeekPosBeyondEof,
                    &format!("tried to set offset beyond EOF, at offset: {}", offset),
                ))
            }
            Some(x) => x,
        };
        Ok(pos.unwrap() as u64)
    }
}

/// Utility wrapper to pass all necessary references to the lookup methods.
#[derive(Debug)]
pub struct Wrapper<'a> {
    pub global: &'a GlobalOptions,
    pub tag: &'a Tag,
    pub vars: &'a mut Variables,
    pub logfile_counter: &'a mut HitCounter,
}

impl<'a> Wrapper<'a> {
    /// Simple helper for creating a new wrapper
    pub fn new(
        global: &'a GlobalOptions,
        tag: &'a Tag,
        vars: &'a mut Variables,
        logfile_counter: &'a mut HitCounter,
    ) -> Self {
        Wrapper {
            global,
            tag,
            vars,
            logfile_counter,
        }
    }
}

/// Return type for all `Lookup` methods.
//pub type LookupRet = Result<Vec<ChildData>, AppError>;

/// Trait, implemented by `LogFile` to search patterns.
pub trait Lookup {
    fn lookup(&mut self, wrapper: &mut Wrapper) -> Result<Vec<ChildData>, AppError>;
    fn lookup_from_reader<R: BufRead + Seeker>(
        &mut self,
        reader: R,
        wrapper: &mut Wrapper,
    ) -> Result<Vec<ChildData>, AppError>;
}

impl Lookup for LogFile {
    ///Just a wrapper function for a file.
    fn lookup(&mut self, wrapper: &mut Wrapper) -> Result<Vec<ChildData>, AppError> {
        // open target file
        let file = File::open(&self.path)?;

        // if file is compressed, we need to call a specific reader
        if self.is_gzipped() {
            let decoder = GzDecoder::new(file);
            let reader = BufReader::new(decoder);
            self.lookup_from_reader(reader, wrapper)
        } else {
            let reader = BufReader::new(file);
            self.lookup_from_reader(reader, wrapper)
        }
    }

    /// The main function of the whole process. Reads a logfile and tests for each line if it matches the regexes.
    ///
    /// Detailed design:
    ///
    /// 1. initialize local variables
    ///     - buffer which will hold read data from each line
    ///     - a `Child` vector which will receive its value from the optional call to a spawned script
    ///     - line and bytes read counters whichkeep track of current line and current number of bytes read
    ///
    /// 2. reset `RunData` fields depending on local options
    ///     - get a mutable reference on `RunData` structure
    ///     - reset thresholds if `savethresholdcount` is set: those thresholds trigger a callback whenever they are reached
    ///     - set current file pointers (offset and line number) to the last ones recorded in the `RunData` structure. If local option
    ///       is set to `rewind`, read from the beginning of the file and set offsets accordingly
    ///
    /// 3. loop to read each line of the file
    ///     - read a line as a byte Vec and convert (lossy) to UTF-8
    ///     - test if each line matches a pattern
    ///     - if yes:
    ///         - test if thresholds are reached. If not loop
    ///         - add rumtime variables, only related to the current line, pattern etc
    ///         - if a script is defined to be called, call the script and save the `Child` return structure

    fn lookup_from_reader<R: BufRead + Seeker>(
        &mut self,
        mut reader: R,
        wrapper: &mut Wrapper,
    ) -> Result<Vec<ChildData>, AppError> {
        //------------------------------------------------------------------------------------
        // 1. initialize local variables
        //------------------------------------------------------------------------------------
        trace!(
            "#######################> start processing logfile:{} for tag:{}",
            self.path.display(),
            wrapper.tag.name()
        );

        // uses the same buffer
        let mut buffer = Vec::with_capacity(Cons::DEFAULT_STRING_CAPACITY);

        // define a new child handle. This is an Option because the script couldn't be called if not requested so
        let mut children = Vec::new();

        // initialize line & byte counters
        let mut bytes_count = 0;
        let mut line_number = 0;

        // to keep handles: stream etc
        let mut handle = CallbackHandle::default();

        // sometimes, early return due to callback errors or I/O errors
        let mut early_ret: Option<AppError> = None;

        //------------------------------------------------------------------------------------
        // 2. reset `RunData` fields depending on local options
        //------------------------------------------------------------------------------------

        // anyway, reset only runtime variables
        wrapper.vars.clear();
        wrapper.vars.insert(
            "LOGFILE",
            self.path.to_str().unwrap_or("error converting PathBuf"),
        );
        wrapper.vars.insert("TAG", wrapper.tag.name());

        // get run_data corresponding to tag name, or insert that new one if not yet in the snapshot file
        let mut run_data = self.rundata(&wrapper.tag.name());
        trace!(
            "tagname: {:?}, run_data:{:?}\n\n",
            &wrapper.tag.name(),
            run_data
        );

        // if we don't need to read the file from the beginning, adjust counters and set offset
        if !wrapper.tag.options.rewind {
            bytes_count = run_data.last_offset;
            line_number = run_data.last_line;
            reader.set_offset(run_data.last_offset)?;
        }

        info!(
            "starting read from last offset={}, last line={}",
            bytes_count, line_number
        );

        // reset exec count
        run_data.exec_count = 0;

        //------------------------------------------------------------------------------------
        // 3. loop to read each line of the file
        //------------------------------------------------------------------------------------
        loop {
            // reset runtime variables because they change for every line read, apart from CLF_LOGFILE
            // which is the same for each log
            //wrapper.vars.retain_logfile();
            wrapper.vars.retain(&[&var!("LOGFILE"), &var!("TAG")]);

            // read until \n (which is included in the buffer)
            let ret = reader.read_until(b'\n', &mut buffer);

            // to deal with UTF-8 conversion problems, use the lossy method. It will replace non-UTF-8 chars with ?
            let mut line = String::from_utf8_lossy(&buffer);

            // remove newline
            line.to_mut().pop();

            // and line feed for Windows platforms
            #[cfg(target_family = "windows")]
            line.to_mut().pop();

            // read_line() returns a Result<usize>
            match ret {
                Ok(bytes_read) => {
                    // EOF: save last file address to restart from this address for next run
                    if bytes_read == 0 {
                        break;
                    }

                    // we've been reading a new line successfully
                    line_number += 1;
                    bytes_count += bytes_read as u64;

                    // do we just need to go to EOF ?
                    if wrapper.tag.options.fastforward {
                        buffer.clear();
                        continue;
                    }

                    trace!("====> line#={}, line={}", line_number, line);

                    // is there a match, regarding also exceptions?
                    if let Some(re) = wrapper.tag.is_match(&line) {
                        debug!(
                            "found a match tag={}, line={}, line#={}, re=({:?},{}), warning_count={}, critical_count={}",
                            wrapper.tag.name(),
                            line.clone(),
                            line_number,
                            re.0,
                            re.1.as_str(),
                            run_data.warning_count,
                            run_data.critical_count
                        );

                        // increments thresholds and compare with possible defined limits and accumulate counters for plugin output
                        match re.0 {
                            PatternType::critical => {
                                run_data.critical_count += 1;
                                if run_data.critical_count < wrapper.tag.options.criticalthreshold {
                                    buffer.clear();
                                    continue;
                                }
                                //wrapper.global_counter.critical_count += 1;
                                wrapper.logfile_counter.critical_count += 1;
                            }
                            PatternType::warning => {
                                run_data.warning_count += 1;
                                if run_data.warning_count < wrapper.tag.options.warningthreshold {
                                    buffer.clear();
                                    continue;
                                }
                                //wrapper.global_counter.warning_count += 1;
                                wrapper.logfile_counter.warning_count += 1;
                            }
                            // this special Ok pattern resets counters
                            PatternType::ok => {
                                run_data.critical_count = 0;
                                run_data.warning_count = 0;

                                // no need to process further: don't call a script
                                buffer.clear();
                                continue;
                            }
                        };

                        // if we've been asked to trigger the script, first add relevant variables
                        if wrapper.tag.options.runcallback {
                            // create variables which will be set as environment variables when script is called
                            wrapper
                                .vars
                                .insert("LINE_NUMBER", format!("{}", line_number));
                            wrapper.vars.insert("LINE", line.clone());
                            wrapper.vars.insert("MATCHED_RE", re.1.as_str());
                            wrapper.vars.insert("MATCHED_RE_TYPE", re.0);

                            wrapper.vars.insert_captures(re.1, &line);

                            debug!("added variables: {:?}", wrapper.vars);

                            // now call script if upper run limit is not reached yet
                            if run_data.exec_count < wrapper.tag.options.runlimit {
                                // in case of a callback error, stop iterating and save state here
                                match wrapper.tag.callback_call(
                                    Some(&wrapper.global.path()),
                                    wrapper.vars,
                                    &mut handle,
                                ) {
                                    Ok(child) => {
                                        // save child structure
                                        if child.is_some() {
                                            children.push(child.unwrap());
                                        }

                                        // increment number of script executions or number of JSON data sent
                                        run_data.exec_count += 1;
                                    }
                                    Err(e) => {
                                        error!(
                                            "error <{}> when calling callback <{:#?}>",
                                            e,
                                            wrapper.tag.callback()
                                        );
                                        early_ret = Some(e);
                                        break;
                                    }
                                };
                            }
                        };
                    }

                    // reset buffer to not accumulate data
                    buffer.clear();
                }
                // a rare IO error could occur here
                Err(e) => {
                    error!("read_line() error kind: {:?}, line: {}", e.kind(), line);
                    early_ret = Some(AppError::Io(e));
                    break;
                }
            };
        }

        // save current offset and line number
        run_data.last_offset = bytes_count;
        run_data.last_line = line_number;

        // resets thresholds if requested
        // this will count number of matches for warning & critical, to see if this matches the thresholds
        // first is warning, second is critical
        if !wrapper.tag.options.savethresholdcount {
            run_data.critical_count = 0;
            run_data.warning_count = 0;
        }

        // and last run
        let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        run_data.last_run = time.as_secs();

        info!("number of callback execution: {}", run_data.exec_count);
        trace!("logfile_counter: {:?}", &wrapper.logfile_counter);
        trace!(
            "========================> end processing logfile:{} for tag:{}",
            self.path.display(),
            wrapper.tag.name()
        );

        // return error if we got one or the list of children from calling the script
        if early_ret.is_some() {
            Err(early_ret.unwrap())
        } else {
            Ok(children)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::str::FromStr;

    use super::*;

    use crate::testing::setup::*;

    // useful set of data for our unit tests
    const JSON: &'static str = r#"
       {
           "/usr/bin/zip": {
                "path": "/usr/bin/zip",
                "compressed": false, 
                "inode": 1,
                "dev": 1,
                "run_data": {
                    "tag1": {
                        "tag_name": "tag1",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000,
                        "critical_count": 10,
                        "warning_count": 10,
                        "exec_count": 10
                    },
                    "tag2": {
                        "tag_name": "tag2",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000,
                        "critical_count": 10,
                        "warning_count": 10,
                        "exec_count": 10
                    }
                }
            },
            "/etc/hosts.allow": {
                "path": "/etc/hosts.allow",
                "compressed": false, 
                "inode": 1,
                "dev": 1,
                "run_data": {
                    "tag3": {
                        "tag_name": "tag3",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000,
                        "critical_count": 10,
                        "warning_count": 10,
                        "exec_count": 10

                    },
                    "tag4": {
                        "tag_name": "tag4",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000,
                        "critical_count": 10,
                        "warning_count": 10,
                        "exec_count": 10
                    }
                }
            }
        }
    "#;

    // fn load_json() -> HashMap<PathBuf, LogFile> {
    //     serde_json::from_str(JSON).unwrap()
    // }

    #[test]
    #[cfg(target_os = "linux")]
    fn new() {
        let mut lf_ok = LogFile::new("/usr/bin/zip").unwrap();
        assert_eq!(lf_ok.path.to_str(), Some("/usr/bin/zip"));
        assert!(lf_ok.extension.is_none());

        lf_ok = LogFile::new("/etc/hosts.allow").unwrap();
        assert_eq!(lf_ok.path.to_str(), Some("/etc/hosts.allow"));
        assert_eq!(lf_ok.extension.unwrap(), "allow");

        // file not found
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
    #[test]
    fn deserialize() {
        let mut json: HashMap<PathBuf, LogFile> = load_json(&JSON);

        assert!(json.contains_key(&PathBuf::from("/usr/bin/zip")));
        assert!(json.contains_key(&PathBuf::from("/etc/hosts.allow")));

        {
            let logfile1 = json.get_mut(&PathBuf::from("/usr/bin/zip")).unwrap();
            assert_eq!(logfile1.run_data.len(), 2);
            assert!(logfile1.contains_key("tag1"));
            assert!(logfile1.contains_key("tag2"));
        }

        {
            let logfile2 = json.get_mut(&PathBuf::from("/etc/hosts.allow")).unwrap();
            assert_eq!(logfile2.run_data.len(), 2);
            assert!(logfile2.contains_key("tag3"));
            assert!(logfile2.contains_key("tag4"));
        }
    }

    #[test]
    fn rundata() {
        let mut json: HashMap<PathBuf, LogFile> = load_json(&JSON);

        {
            let rundata1 = json
                .get_mut(&PathBuf::from("/usr/bin/zip"))
                .unwrap()
                .rundata("another_tag");

            assert_eq!(rundata1.tag_name, "another_tag");
        }

        let logfile1 = json.get_mut(&PathBuf::from("/usr/bin/zip")).unwrap();
        let mut tags = logfile1.tags();
        tags.sort();
        assert_eq!(tags, vec!["another_tag", "tag1", "tag2"]);

        assert!(logfile1.contains_key("tag1"));
        assert!(!logfile1.contains_key("tag3"));
    }

    fn get_compressed_reader() -> BufReader<GzDecoder<File>> {
        let file = File::open("tests/logfiles/clftest.txt.gz")
            .expect("unable to open compressed test file");
        let decoder = GzDecoder::new(file);
        let reader = BufReader::new(decoder);

        reader
    }

    #[test]
    fn set_offset() {
        let mut buffer = [0; 1];

        let mut reader = get_compressed_reader();
        let mut offset = reader.set_offset(1);
        assert!(offset.is_ok());
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer[0], 'B' as u8);

        reader = get_compressed_reader();
        offset = reader.set_offset(0);
        assert!(offset.is_ok());
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer[0], 'A' as u8);

        reader = get_compressed_reader();
        offset = reader.set_offset(26);
        assert!(offset.is_ok());
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer[0], '\n' as u8);

        reader = get_compressed_reader();
        offset = reader.set_offset(25);
        assert!(offset.is_ok());
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer[0], 'Z' as u8);

        reader = get_compressed_reader();
        offset = reader.set_offset(10000);
        assert!(offset.is_err());
    }

    #[test]
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
        let mut vars = Variables::default();
        let mut logfile_counter = HitCounter::default();

        let mut w = Wrapper::new(&opts, &tag, &mut vars, &mut logfile_counter);

        let mut logfile = LogFile::new("tests/logfiles/adhoc.txt").unwrap();

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

        let ret = logfile.lookup(&mut w);
        let _res = child.join();
    }
}
