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

use std::fmt::Display;
use std::fs::File;
use std::path::{Path, PathBuf};

use log::debug;
//use regex::Regex;
use serde::{de, Deserialize, Deserializer};
use serde_yaml::Value;

use crate::config::{
    callback::{Callback, CallbackHandle, ChildData},
    options::SearchOptions,
    pattern::{PatternMatchResult, PatternSet},
    vars::{RuntimeVars, UserVars},
};

use crate::misc::{
    error::AppError,
    util::{Cons, Util},
};

use crate::logfile::snapshot::Snapshot;

/// Auto-implement the FromStr trait for a struct
#[macro_export]
macro_rules! fromstr {
    ($t:ty) => {
        impl std::str::FromStr for $t {
            type Err = serde_yaml::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                serde_yaml::from_str(s)
            }
        }
    };
}

#[derive(Debug, Deserialize, Clone)]
/// A list of global options, which apply globally for all searches.
#[serde(default)]
pub struct GlobalOptions {
    /// A list of paths, separated by either ':' for unix, or ';' for Windows. This is
    /// where the script, if any, will be searched for. Default to PATH or Path depending on the platform.
    path: String,

    /// A directory where matched lines will be stored.
    output_dir: PathBuf,

    /// The snapshot file name. Option<> is used because if not specified here,
    snapshot_file: PathBuf,

    /// Retention time for tags.
    snapshot_retention: u64,

    /// A list of user variables if any.
    user_vars: Option<UserVars>,
}

impl GlobalOptions {
    #[inline(always)]
    pub fn path(&self) -> &str {
        &self.path
    }

    // Returns the user variables if any. Clone of the original HashMap.
    #[inline(always)]
    pub fn user_vars(&self) -> &Option<UserVars> {
        &self.user_vars
    }
}

// Auto-implement FromStr
fromstr!(GlobalOptions);

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
            snapshot_file: Snapshot::default_name(),
            snapshot_retention: Cons::DEFAULT_RETENTION,
            user_vars: None,
        }
    }
}

/// A `enum` matching either a logfile name if only a single logfile is defined, or a list
/// of logfile names is case of command is given. This command is expected to return to the
/// the standard output the list of files to check. One of the enum variant is loaded from
/// the YAML configuration file.
#[derive(Debug, Deserialize, Clone)]
pub enum LogSource {
    #[serde(rename = "logfile")]
    LogFile(PathBuf),

    #[serde(rename = "loglist")]
    LogList {
        cmd: String,
        args: Option<Vec<String>>,
    },
}

impl LogSource {
    pub const fn is_logfile(&self) -> bool {
        matches!(*self, LogSource::LogFile(_))
        // match self {
        //     LogSource::LogFile(_) => true,
        //     LogSource::LogList { cmd: _, args: _ } => false,
        // }
    }
}

impl Display for LogSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogSource::LogFile(logfile) => write!(f, "{}", logfile.display()),
            LogSource::LogList { cmd: _, args: _ } => {
                unimplemented!("LogSource::LogList not permitted !")
            }
        }
    }
}

/// This is the core structure which handles data used to search into the logfile. These are
/// gathered and refered to a tag name.
#[derive(Debug, Deserialize, Clone)]
pub struct Tag {
    /// A name to identify the tag.
    name: String,

    /// Tells whether we process this tag or not. Useful for testing purposes.
    #[serde(default = "Tag::default_process")]
    process: bool,

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
    /// Returns the tag name
    #[inline(always)]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the process value
    #[inline(always)]
    pub fn process(&self) -> bool {
        self.process
    }

    /// Returns the callback field
    #[inline(always)]
    pub fn callback(&self) -> &Option<Callback> {
        &self.callback
    }

    /// Returns the regex involved in a match, if any, along with associated the pattern type.
    pub fn is_match(&self, text: &str) -> Option<PatternMatchResult> {
        self.patterns.is_match(text)
    }

    /// Default value for processing a tag
    pub fn default_process() -> bool {
        true
    }

    /// Calls the external callback, by providing arguments, environment variables and path which will be searched for the command.
    pub fn callback_call(
        &self,
        path: Option<&str>,
        user_vars: &Option<UserVars>,
        runtime_vars: &RuntimeVars,
        handle: &mut CallbackHandle,
    ) -> Result<Option<ChildData>, AppError> {
        if self.callback.is_some() {
            self.callback
                .as_ref()
                .unwrap()
                .call(path, user_vars, runtime_vars, handle)
        } else {
            Ok(None)
        }
    }
}

// Auto-implement FromStr
fromstr!(Tag);

/// This structure keeps everything related to log rotation
#[derive(Debug, Deserialize, Clone)]
pub struct LogArchive {
    /// the logfile name to check
    dir: Option<PathBuf>,
    ext: String, //archive_regex:
}

impl LogArchive {
    pub fn archived_path<P: AsRef<Path>>(&self, path: P) -> Option<PathBuf> {
        // if we don't specify a directory for archive, it'll use the same as path
        if self.dir.is_none() {
            let mut new_path = path.as_ref().to_path_buf();
            new_path.push(&self.ext);

            Some(new_path)
        }
        // otherwise use it
        else {
            if let Some(file_name) = path.as_ref().file_name() {
                let mut new_path = PathBuf::new();
                new_path.push(self.dir.as_ref().unwrap());
                new_path.push(file_name);
                new_path.set_extension(self.ext.clone());

                Some(new_path)
            } else {
                None
            }
        }
    }
}

/// This is the structure mapping exactly search data coming from the configuration YAML file. The 'flatten' serde field
/// attribute allows to either use a logfile name or a command.
#[derive(Debug, Deserialize, Clone)]
pub struct Search {
    /// the logfile name to check
    #[serde(flatten)]
    pub logfile: LogSource,

    /// log rotation settings
    pub archive: Option<LogArchive>,

    /// a unique identifier for this search
    pub tags: Vec<Tag>,
}

impl Search {
    /// Returns the `LogSource::LogFile` variant which corresponds to a logfile
    pub fn logfile(&self) -> &PathBuf {
        match &self.logfile {
            LogSource::LogFile(logfile) => &logfile,
            LogSource::LogList { cmd: _, args: _ } => {
                unimplemented!("LogSource::LogList not permitted !")
            }
        }
    }
}

/// The main search configuration used to search patterns in a logfile. This is loaded from
/// the YAML file found in the command line argument (or from stdin). This configuration can include a list
/// of logfiles (given either by name or by starting an external command) to lookup and for each logfile, a list of regexes to match.
#[derive(Debug, Deserialize, Default)]
pub struct Config {
    /// List of global options, which apply for all searches.
    #[serde(default = "GlobalOptions::default")]
    global: GlobalOptions,

    /// list of searches.
    #[serde(deserialize_with = "fill_log")]
    pub searches: Vec<Search>,
}

impl Config {
    /// Returns a reference on `global` fields.
    #[inline(always)]
    pub fn global(&self) -> &GlobalOptions {
        &self.global
    }

    /// Sets the snapshot file if provided
    pub fn set_snapshot_file(&mut self, snapfile: &PathBuf) {
        self.global.snapshot_file = snapfile.to_path_buf();
    }

    /// Returns the name of the snapshot file
    #[inline(always)]
    pub fn snapshot_file(&self) -> &PathBuf {
        &self.global.snapshot_file
    }

    /// Returns the snapshot retention
    #[inline(always)]
    pub fn snapshot_retention(&self) -> u64 {
        self.global.snapshot_retention
    }
}

// Auto-implement FromStr
fromstr!(Config);

impl Config {
    /// Loads a YAML configuration file as a `Config` struct.
    pub fn from_path<P: AsRef<Path>>(file_name: P) -> Result<Config, AppError> {
        // open YAML file
        let file = File::open(file_name)?;

        // load YAML data
        let yaml: Config = serde_yaml::from_reader(file)?;

        debug!(
            "sucessfully loaded YAML configuration file, nb_searches={}",
            yaml.searches.len()
        );
        Ok(yaml)
    }
}

/// Replace the `logsource` YAML tag with the result of the script command
fn fill_log<'de, D>(deserializer: D) -> Result<Vec<Search>, D::Error>
where
    D: Deserializer<'de>,
{
    // get the YAML `Value` from serde. See https://docs.serde.rs/serde_yaml/enum.Value.html
    let yaml_value: Value = Deserialize::deserialize(deserializer)?;

    // transform this value into our struct
    let vec_yaml: Result<Vec<Search>, _> =
        serde_yaml::from_value(yaml_value).map_err(de::Error::custom);
    if vec_yaml.is_err() {
        return vec_yaml;
    }
    let mut vec_search = vec_yaml.unwrap();

    // this vector wil hold new logfiles from the list returned from the script execution
    let mut vec_loglist: Vec<Search> = Vec::new();

    for search in &vec_search {
        match &search.logfile {
            // we found a logfile tag: just copy everything to the new structure
            LogSource::LogFile(_) => continue,

            // we found a logslist tag: get the list of files, and for each one, copy everything
            LogSource::LogList {
                cmd: _cmd,
                args: _args,
            } => {
                // get optional arguments
                let script_args = _args.as_ref().map(|f| f.as_slice());

                // get list of files from command or script
                let files = Util::get_list(_cmd, script_args).unwrap();

                //println!("returned files: {:?}", files);

                // create Search structure with the files we found, and a clone of all tags
                for file in &files {
                    // create a new Search structure based on the file we just found
                    let search_pathbuf = Search {
                        logfile: LogSource::LogFile(file.to_path_buf()),
                        archive: search.archive.clone(),
                        tags: search.tags.clone(),
                    };

                    // now use this structure and add it to config_pathbuf
                    vec_loglist.push(search_pathbuf);
                }
            }
        }
    }

    // add all those new logfiles we found
    vec_search.extend(vec_loglist);

    // keep only valid logfiles, not logsources
    vec_search.retain(|x| x.logfile.is_logfile());
    Ok(vec_search)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn archived_path() {
        let rot = LogArchive {
            dir: Some(PathBuf::from("/tmp")),
            ext: String::from("gz"),
        };

        let path = rot.archived_path("/var/log/kern.log");
        assert!(path.is_some());
        assert_eq!(path.unwrap(), PathBuf::from("/tmp/kern.log.gz"));
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn global_options() {
        let mut yaml = r#"
path: /usr/foo1
snapshot_file: /usr/foo3/snap.foo
output_dir: /usr/foo2
        "#;

        let mut opts = GlobalOptions::from_str(yaml).expect("unable to read YAML");
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

        opts = GlobalOptions::from_str(yaml).expect("unable to read YAML");
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

        let tag: Tag = Tag::from_str(yaml).expect("unable to read YAML");
        assert_eq!(tag.name, "error");
        assert!(tag.options.runcallback);
        assert!(!tag.options.keepoutput);
        assert!(!tag.process);
        let script = PathBuf::from("tests/scripts/echovars.py");
        assert!(
            matches!(&tag.callback.as_ref().unwrap().callback, crate::config::callback::CallbackType::Script(Some(x)) if x == &script)
        );
        assert_eq!(
            tag.callback.unwrap().args.unwrap(),
            &["arg1", "arg2", "arg3"]
        );
    }

    #[test]
    fn config() {
        dbg!(std::env::current_dir().unwrap());
        let yaml = r#"
        global:
          path: "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
          output_dir: /tmp/foo
          snapshot_file: /tmp/my_snapshot.json
          snapshot_retention: 5
          user_vars:
            first_name: Al
            last_name: Pacino
            city: 'Los Angeles'
            profession: actor
        
        searches:
          - logfile: tests/logfiles/small_access.log
            tags: 
              - name: http_access_get_or_post
                process: true
                options: "warningthreshold=0"
                callback: { 
                  script: "tests/scripts/echovars.py",
                  args: ['arg1', 'arg2', 'arg3']
                }
                patterns:
                  critical: {
                    regexes: [
                      'GET\s+([/\w]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)',
                    ],
                  }
                  warning: {
                    regexes: [
                      'POST\s+([/\w\.]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)'
                    ],
                    exceptions: [
                      '^\d{2,3}\.'
                    ]
                  }        
        "#;
        let _config = Config::from_str(yaml).expect("unable to read YAML");
        let config = Config::from(_config);

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
            search.logfile(),
            &PathBuf::from("tests/logfiles/small_access.log")
        );
        assert_eq!(search.tags.len(), 1);

        let tag = search.tags.first().unwrap();
        assert_eq!(&tag.name, "http_access_get_or_post");
        assert!(tag.process);
        assert_eq!(tag.options.warningthreshold, 0);
        assert!(tag.callback.is_some());
        let script = PathBuf::from("tests/scripts/echovars.py");
        assert!(
            matches!(&tag.callback.as_ref().unwrap().callback, crate::config::callback::CallbackType::Script(Some(x)) if x == &script)
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
