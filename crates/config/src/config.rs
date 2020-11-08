//! Holds the main configuration data, loaded from a YAML file.
//!
//! This YAML file is divided into 2 parts:
//!
//! * a `global` YAML structure mapping the `Global` Rust structure which holds options which apply for each search
//! * an array of searches (the `searches`) tag which describes which files to search for, and the patterns which might
//! trigger a match.
//!
//! The logfile could either be an accessible file path, or a command which will be executed and gets back a list of files.
//!
//!
//!

//!

//use std::convert::TryFrom;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use log::{debug, error, info};
use regex::Regex;
use serde::Deserialize;

use crate::{
    callback::{Callback, CallbackHandle, ChildData},
    pattern::{PatternSet, PatternType},
    variables::Variables,
};

use misc::{
    error::{AppCustomErrorKind, AppError},
    nagios::NagiosError,
    util::Util,
};

/// A default value for the retention of data in the snapshot file.
const DEFAULT_RETENTION: u64 = 86000 * 7;

#[derive(Debug, Deserialize, Clone)]
/// A list of global options, which apply globally for all searches.
#[serde(default)]
pub struct GlobalOptions {
    /// A list of paths, separated by either ':' for unix, or ';' for Windows. This is
    /// where the script, if any, will be searched for. Default to PATH or Path depending on the platform.
    pub path: String,

    /// A directory where matched lines will be stored.
    output_dir: PathBuf,

    /// The snapshot file name. Option<> is used because if not specified here,
    snapshot_file: PathBuf,

    /// Retention time for tags.
    snapshot_retention: u64,

    /// A list of user variables if any.
    user_vars: Option<HashMap<String, String>>,
}

/// Default implementation, rather than serde default field attribute.
impl Default for GlobalOptions {
    fn default() -> Self {
        // default path
        let path_var = if cfg!(target_family = "unix") {
            std::env::var("PATH").unwrap_or_else(|_| "/usr/sbin:/usr/bin:/sbin:/bin".to_string())
        } else if cfg!(target_family = "windows") {
            std::env::var("Path").unwrap_or_else(|_| {
                r"C:\Windows\system32;C:\Windows;C:\Windows\System32\Wbem;".to_string()
            })
        } else {
            unimplemented!("unsupported OS, file: {}:{}", file!(), line!());
        };

        // default logger path
        let mut logger_path = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
        logger_path.push("clf.log");

        GlobalOptions {
            path: path_var,
            output_dir: std::env::temp_dir(),
            snapshot_file: Util::snapshot_default_name(),
            snapshot_retention: DEFAULT_RETENTION,
            user_vars: None,
        }
    }
}

/// A list of options which are specific to a search. They might or might not be used. If an option is not present, it's deemed false.
/// By default, all options are either false, or use the default corresponding type.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(try_from = "String")]
pub struct SearchOptions {
    /// If `true`, the defined script will be run a first match.
    pub runcallback: bool,

    /// If `true`, the matching line will be saved in an output file.
    pub keepoutput: bool,

    /// If `true`, the logfile will be search from the beginning, regardless of any saved offset.
    pub rewind: bool,

    /// a number which denotes how many lines have to match a pattern until they are considered a critical error
    pub criticalthreshold: u16,

    /// a number which denotes how many lines have to match a pattern until they are considered a warning
    pub warningthreshold: u16,

    // controls whether the matching lines are written to a protocol file for later investigation
    pub protocol: bool,

    /// controls whether the hit counter will be saved between the runs.
    /// If yes, hit numbers are added until a threshold is reached (criticalthreshold).
    /// Otherwise the run begins with reset counters
    pub savethresholdcount: bool,

    /// controls whether an error is propagated through successive runs of check_logfiles.
    /// Once an error was found, the exitcode will be non-zero until an okpattern resets it or until
    /// the error expires after <second> seconds. Do not use this option until you know exactly what you do
    pub sticky: u16,

    /// Moves to the end of the file for the first read, if the file has not been yet read.
    pub fastforward: bool,

    /// The number of times a potential script will be called, at most.
    pub runlimit: u16,
}

/// Convenient macro to add a boolean option
macro_rules! add_bool_option {
    ($v:ident, $opt:ident, $($bool_option:ident),*) => (
        $(
          if $v.contains(&stringify!($bool_option)) {
            $opt.$bool_option = true;
        }
        )*
    );
}

/// Convenient macro to add an integer or string option.
macro_rules! add_typed_option {
    // add non-boolean option if any. It converts to the target type
    ($x:ident, $tag:ident, $opt:ident, $type:ty) => {
        // `stringify!` will convert the expression *as it is* into a string.
        if $x[0] == stringify!($tag) {
            $opt.$tag = $x[1].parse::<$type>().unwrap();
        }
    };
}

/// Converts a list of comma-separated options to a `SearchOptions` structure.
impl TryFrom<String> for SearchOptions {
    type Error = AppError;

    fn try_from(option_list: String) -> Result<Self, Self::Error> {
        // list of valid options
        const VALID_OPTIONS: &[&str] = &[
            "runcallback",
            "keepoutput",
            "rewind",
            "criticalthreshold",
            "warningthreshold",
            "protocol",
            "savethresholdcount",
            "sticky",
            "fastforward",
            "runlimit",
        ];

        // create a default options structure
        let mut opt = SearchOptions::default();

        // runlimit is special
        opt.runlimit = std::u16::MAX;

        // convert the input list to a vector
        let opt_list: Vec<_> = option_list.split(',').map(|x| x.trim()).collect();

        // checks if there're any invalid arguments
        for opt in &opt_list {
            if VALID_OPTIONS.iter().all(|x| !opt.contains(x)) {
                return Err(AppError::App {
                    err: AppCustomErrorKind::UnsupportedSearchOption,
                    msg: format!("search option: {}  is not supported", opt),
                });
            }
        }

        // use Rust macro to add bool options if any
        add_bool_option!(
            opt_list,
            opt,
            runcallback,
            rewind,
            keepoutput,
            savethresholdcount,
            protocol
        );

        // other options like key=value if any
        // first build a vector of such options. We first search for = and then split according to '='
        let kv_options: Vec<_> = opt_list.iter().filter(|&x| x.contains('=')).collect();

        // need to test whether we found 'key=value' options
        if !kv_options.is_empty() {
            // this hash will hold key values options
            //let kvh_options: HashMap<String, String> = HashMap::new();

            // now we can safely split
            for kv in &kv_options {
                let splitted_options: Vec<_> = kv.split('=').map(|x| x.trim()).collect();
                let _key = splitted_options[0];
                let _value = splitted_options[1];

                // add additional non-boolean options if any
                add_typed_option!(splitted_options, criticalthreshold, opt, u16);
                add_typed_option!(splitted_options, sticky, opt, u16);
                add_typed_option!(splitted_options, warningthreshold, opt, u16);
                add_typed_option!(splitted_options, runlimit, opt, u16);

                // special case for this
                // if key == "logfilemissing" {
                //     opt.logfilemissing = LogfileMissing::from_str(value).unwrap();
                // }
            }
        }

        Ok(opt)
    }
}

/// A `enum` matching either a logfile name if only a single logfile is defined, or a list
/// of logfile names is case of command is given. This command is expected to return to the
/// the standard output the list of files to check. One of the enum variant is loaded from
/// the YAML configuration file.
#[derive(Debug, Deserialize, Clone)]
pub enum LogSource {
    #[serde(rename = "logfile")]
    LogFile(String),

    #[serde(rename = "loglist")]
    LogList {
        cmd: String,
        args: Option<Vec<String>>,
    },
}

/// This is the core structure which handles data used to search into the logfile. These are
/// gathered and refered to a tag name.
#[derive(Debug, Deserialize, Clone)]
pub struct Tag {
    /// A name to identify the tag.
    pub name: String,

    /// Tells whether we process this tag or not. Useful for testing purposes.
    #[serde(default = "Tag::process_default")]
    pub process: bool,

    /// A list of options specific to this search. As such options are optional, add a default `serde`
    /// directive.
    #[serde(default = "SearchOptions::default")]
    pub options: SearchOptions,

    /// Script details like path, name, parameters, delay etc to be possibly run for a match.
    callback: Option<Callback>,

    /// Patterns to be checked against. These include critical and warning (along with exceptions), ok list of regexes.
    patterns: PatternSet,
}

impl Tag {
    /// Returns the regex involved in a match, if any, along with associated the pattern type.
    pub fn is_match(&self, text: &str) -> Option<(PatternType, &Regex)> {
        self.patterns.is_match(text)
    }

    ///
    pub fn process_default() -> bool {
        true
    }

    // Calls the external script, by providing arguments, environment variables and path which will be searched for the command.
    // pub fn call_script(
    //     &self,
    //     path: Option<&str>,
    //     vars: &Variables,
    // ) -> Result<Option<ChildData>, AppError> {
    //     // if the script tag has been defined...
    //     if let Some(callback) = &self.script {
    //         // if callback is a script to spawn, call it
    //         if callback.path.is_some() {
    //             let child = callback.spawn(path, vars)?;
    //             Ok(Some(child))
    //         // if a network script is defined, send it data through JSON
    //         } else if callback.address.is_some() {
    //             let _res = callback.send(vars)?;
    //             return Ok(None);
    //         } else {
    //             Ok(None)
    //         }
    //     } else {
    //         Ok(None)
    //     }
    // }
    // Calls the external callback, by providing arguments, environment variables and path which will be searched for the command.
    pub fn callback_call(
        &self,
        path: Option<&str>,
        vars: &mut Variables,
        handle: &mut CallbackHandle,
    ) -> Result<Option<ChildData>, AppError> {
        if self.callback.is_some() {
            self.callback.as_ref().unwrap().call(path, vars, handle)
        } else {
            Ok(None)
        }
    }
}

/// This is the structure mapping exactly search data coming from the configuration YAML file. The 'flatten' serde field
/// attribute allows to either use a logfile name or a command.
#[derive(Debug, Deserialize, Clone)]
pub struct Search<T: Clone> {
    /// the logfile name to check
    #[serde(flatten)]
    pub logfile: T,

    #[serde(default = "NagiosError::default")]
    pub io_error: NagiosError,

    /// a unique identifier for this search
    pub tags: Vec<Tag>,
    //#[serde(default = "Tag::process_default")]
    //pub process: bool,
}

/// This conversion utility is meant to convert to a 'regular' configuration file a configuration file
/// using the `logfile` YAML tag with a command.
impl From<Search<LogSource>> for Search<PathBuf> {
    fn from(search_logsource: Search<LogSource>) -> Self {
        // if LogSource::Logfile, just copy. Otherwise, it's unimplemented
        let logfile = match search_logsource.logfile {
            LogSource::LogFile(lf) => PathBuf::from(lf),
            //LogSource::LogList { cmd: _, args: _ } => PathBuf::from(""),
            _ => unimplemented!("this could not occur"),
        };

        Search {
            logfile: logfile.clone(),
            io_error: search_logsource.io_error.clone(),
            tags: search_logsource.tags.clone(),
            //process: search_logsource.process.clone(),
        }
    }
}

/// Builds a default logger file.
// pub fn default_logger() -> PathBuf {
//     let mut logger_path = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
//     logger_path.push("clf.log");
//     logger_path
// }

/// The main search configuration used to search patterns in a logfile. This is loaded from
/// the YAML file found in the command line argument (or from stdin). This configuration can include a list
/// of logfiles (given either by name or by starting an external command) to lookup and for each logfile, a list of regexes to match.
#[derive(Debug, Deserialize, Default)]
pub struct Config<T: Clone> {
    /// List of global options, which apply for all searches.
    #[serde(default = "GlobalOptions::default")]
    global: GlobalOptions,

    /// list of searches.
    pub searches: Vec<Search<T>>,
}

impl<T: Clone> Config<T> {
    /// Returns a reference on `global` fields.
    #[inline(always)]
    pub fn get_global(&self) -> &GlobalOptions {
        &self.global
    }

    /// Returns the name of the snapshot file
    // #[inline(always)]
    // pub fn get_snapshot_name(&self) -> &PathBuf {
    //     &self.global.snapshot_file
    // }

    // Returns the user variables if any. Clone of the original HashMap.
    #[inline(always)]
    pub fn get_user_vars(&self) -> Option<HashMap<String, String>> {
        self.global.user_vars.clone()
    }

    /// Returns the snapshot retention
    #[inline(always)]
    pub fn get_snapshot_retention(&self) -> u64 {
        self.global.snapshot_retention
    }
}

impl Config<LogSource> {
    /// Loads a YAML configuration string as a `Config` struct.
    pub fn from_str(s: &str) -> Result<Config<LogSource>, AppError> {
        // load YAML data from a string
        let yaml = serde_yaml::from_str(s)?;
        Ok(yaml)
    }

    /// Loads a YAML configuration from a reader as a `Config` struct.
    pub fn from_reader<R: Read>(rdr: R) -> Result<Config<LogSource>, AppError> {
        // load YAML data from a reader
        let yaml = serde_yaml::from_reader(rdr)?;
        Ok(yaml)
    }

    /// Loads a YAML configuration file as a `Config` struct.
    pub fn from_file<P: AsRef<Path>>(file_name: P) -> Result<Config<LogSource>, AppError> {
        // open YAML file
        let file = File::open(file_name)?;

        // load YAML data
        let yaml: Config<LogSource> = serde_yaml::from_reader(file)?;

        debug!(
            "sucessfully loaded YAML configuration file, nb_searches={}",
            yaml.searches.len()
        );
        Ok(yaml)
    }

    // pub fn get_snapshot_name(&self) -> &PathBuf {
    //     &self.global.as_ref().unwrap().snapshotfile
    // }
}

impl From<Config<LogSource>> for Config<PathBuf> {
    /// This conversion utility is meant to add, for each search when used with a command, the tag data defined.
    fn from(config_logsource: Config<LogSource>) -> Self {
        // initialize a default Config structure
        let mut config_pathbuf = Config::<PathBuf>::default();

        // copy global options
        config_pathbuf.global = config_logsource.global.clone();

        // for each Search, clone if LogSource::logfile, or replace by list of files is LogSource::loglist
        for search in &config_logsource.searches {
            match &search.logfile {
                // we found a logfile tag: just copy everything to the new structure
                LogSource::LogFile(_) => {
                    let search_pathbuf = Search::<PathBuf>::from(search.clone());
                    config_pathbuf.searches.push(search_pathbuf);
                }

                // we found a logslist tag: get the list of files, and for each one, copy everything
                LogSource::LogList {
                    cmd: _cmd,
                    args: _args,
                } => {
                    // get optional arguments
                    let script_args = _args.as_ref().map(|f| f.as_slice());

                    // get list of files from command or script
                    let files = match Util::get_list(_cmd, script_args) {
                        Ok(file_list) => {
                            if file_list.is_empty() {
                                info!(
                                    "no files returned by command: {}, with args: {:?}",
                                    _cmd, _args
                                );
                            }
                            file_list
                        }
                        Err(e) => {
                            error!(
                                "error: {} when executing command: {}, args: {:?}",
                                e, _cmd, _args
                            );
                            break;
                        }
                    };
                    debug!("returned files: {:?}", files);

                    // create Search structure with the files we found, and a clone of all tags
                    for file in &files {
                        // create a new Search structure based on the file we just found
                        let search_pathbuf = Search::<PathBuf> {
                            logfile: file.clone(),
                            io_error: search.io_error.clone(),
                            tags: search.tags.clone(),
                            //process: search.process.clone(),
                        };

                        // now use this structure and add it to config_pathbuf
                        config_pathbuf.searches.push(search_pathbuf);
                    }
                }
            }
        }

        config_pathbuf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //use std::path::PathBuf;

    #[test]
    fn search_options() {
        let opts = SearchOptions::try_from("runcallback, keepoutput, rewind, criticalthreshold=10, warningthreshold=15, protocol, savethresholdcount, sticky=5, runlimit=10".to_string()).unwrap();

        assert!(opts.runcallback);
        assert!(opts.keepoutput);
        assert!(opts.rewind);
        assert!(opts.savethresholdcount);
        assert!(opts.protocol);

        assert_eq!(opts.criticalthreshold, 10);
        assert_eq!(opts.warningthreshold, 15);
        assert_eq!(opts.sticky, 5);
        assert_eq!(opts.criticalthreshold, 10);
        assert_eq!(opts.runlimit, 10);
        //assert_eq!(&opts.logfilemissing.unwrap(), "foo");
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn global_options() {
        let mut yaml = r#"
path: /usr/foo1
snapshot_file: /usr/foo3/snap.foo
output_dir: /usr/foo2
        "#;

        let mut opts: GlobalOptions = serde_yaml::from_str(yaml).expect("unable to read YAML");
        //println!("opts={:?}", opts);

        assert_eq!(&opts.path, "/usr/foo1");
        assert_eq!(opts.output_dir, PathBuf::from("/usr/foo2"));
        assert_eq!(opts.snapshot_file, PathBuf::from("/usr/foo3/snap.foo"));

        yaml = r#"
path: /usr/foo1

# a list of user variables, if any
user_vars:
    first_name: Al
    last_name: Pacino
    city: 'Los Angeles'
    profession: actor            
        "#;

        opts = serde_yaml::from_str(yaml).expect("unable to read YAML");
        assert_eq!(&opts.path, "/usr/foo1");
        assert_eq!(opts.output_dir, PathBuf::from("/tmp"));
        assert_eq!(opts.snapshot_file, PathBuf::from("/tmp/clf_snapshot.json"));
        assert!(opts.user_vars.is_some());

        let vars = opts.user_vars.unwrap();
        assert_eq!(vars.get("first_name").unwrap(), "Al");
        assert_eq!(vars.get("last_name").unwrap(), "Pacino");
        assert_eq!(vars.get("city").unwrap(), "Los Angeles");
        assert_eq!(vars.get("profession").unwrap(), "actor");
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn tag() {
        let yaml = r#"
name: error
options: "runcallback"
process: false
callback: { 
    script: "tests/scripts/echovars.py",
    args: ['arg1', 'arg2', 'arg3']
}
patterns:
    warning: {
        regexes: [
            'error',
        ],
        exceptions: [
            'STARTTLS'
        ]
    }
        "#;

        let tag: Tag = serde_yaml::from_str(yaml).expect("unable to read YAML");
        assert_eq!(tag.name, "error");
        assert!(tag.options.runcallback);
        assert!(!tag.options.keepoutput);
        assert!(!tag.process);
        let script = PathBuf::from("tests/scripts/echovars.py");
        assert!(
            matches!(&tag.callback.as_ref().unwrap().callback, crate::callback::CallbackType::Script(Some(x)) if x == &script)
        );
        assert_eq!(
            tag.callback.unwrap().args.unwrap(),
            &["arg1", "arg2", "arg3"]
        );
    }

    #[test]
    fn config() {
        let yaml = include_str!("../tests/config1.yml");
        let _config = Config::<LogSource>::from_str(yaml).expect("unable to read YAML");
        let config = Config::<PathBuf>::from(_config);

        assert_eq!(
            &config.global.path,
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
        );
        assert_eq!(config.global.output_dir, PathBuf::from("/tmp/foo"));
        assert_eq!(
            config.global.snapshot_file,
            PathBuf::from("/tmp/my_snapshot.json")
        );
        assert_eq!(config.global.snapshot_retention, 5);
        assert_eq!(
            &config
                .global
                .user_vars
                .as_ref()
                .unwrap()
                .get("first_name")
                .unwrap(),
            &"Al"
        );
        assert_eq!(
            &config
                .global
                .user_vars
                .as_ref()
                .unwrap()
                .get("last_name")
                .unwrap(),
            &"Pacino"
        );
        assert_eq!(
            &config
                .global
                .user_vars
                .as_ref()
                .unwrap()
                .get("city")
                .unwrap(),
            &"Los Angeles"
        );
        assert_eq!(
            &config
                .global
                .user_vars
                .as_ref()
                .unwrap()
                .get("profession")
                .unwrap(),
            &"actor"
        );
        assert_eq!(config.searches.len(), 1);

        let search = config.searches.first().unwrap();
        assert_eq!(
            search.logfile,
            PathBuf::from("tests/logfiles/small_access.log")
        );
        assert_eq!(search.tags.len(), 1);

        let tag = search.tags.first().unwrap();
        assert_eq!(&tag.name, "http_access_get_or_post");
        assert!(tag.process);
        assert_eq!(tag.options.warningthreshold, 0);
        assert!(tag.callback.is_some());
        let script = PathBuf::from("tests/scripts/echovars.py");
        assert!(
            matches!(&tag.callback.as_ref().unwrap().callback, crate::callback::CallbackType::Script(Some(x)) if x == &script)
        );
        assert_eq!(
            tag.callback.as_ref().unwrap().args.as_ref().unwrap(),
            &["arg1", "arg2", "arg3"]
        );
        assert!(tag.patterns.ok.is_none());
        assert!(tag.patterns.critical.is_some());
        assert!(tag.patterns.warning.is_some());
    }
}
