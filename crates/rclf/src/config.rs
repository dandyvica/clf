//! Holds the main configuration data, loaded from a YAML file.
//use std::convert::TryFrom;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use log::debug;
use regex::Regex;
use serde::Deserialize;

use crate::{
    command::Cmd,
    error::AppError,
    pattern::{PatternSet, PatternType},
    variables::Vars,
};

// #[cfg(target_os = "linux")]
// const SEPARATOR: char = ':';

// #[cfg(target_os = "windows")]
// const SEPARATOR: char = ';';

// /// A list of paths, where the script which is potentially called, are scanned the locate
// /// this script.
// #[derive(Debug, Deserialize)]
// #[serde(from = "String")]
// pub struct PathList(pub Vec<PathBuf>);

// /// Just converts a list of paths separated by either ':' or ';' depending on the platform
// /// to a vector of `PathBuf`.
// ///
// /// # Example
// ///
// /// ```rust
// /// use std::path::PathBuf;
// /// use rclf::config::PathList;
// ///
// /// let pl = PathList::from("/bin:/usr/bin:/usr/loca/bin".to_string());
// /// assert_eq!(pl.0.len(), 3);
// /// ```
// impl From<String> for PathList {
//     fn from(list: String) -> Self {
//         PathList(list.split(SEPARATOR).map(|p| PathBuf::from(p)).collect())
//     }
// }

// /// A helper structure to represent a script or command to be run on each match.
// #[derive(Debug, Deserialize)]
// pub struct Script {
//     /// Name of the script to spawn without its path.
//     pub path: PathBuf,

//     /// list of its optional paths
//     //pub pathlist: Option<String>,

//     /// List of its optional arguments.
//     pub args: Option<Vec<String>>,

//     /// Timeout in seconds after which the script is killed.
//     #[serde(default)]
//     pub timeout: u64,
// }

// impl Script {
//     /// Returns the canonical, absolute form of the path with all intermediate
//     /// components normalized and symbolic links resolved.
//     ///
//     /// # Example
//     ///
//     /// ```rust
//     /// use std::path::PathBuf;
//     /// use rclf::config::Script;
//     ///
//     /// let script = Script {
//     ///     path: PathBuf::from("gzip"),
//     ///     args: None,
//     ///     timeout: 0
//     /// };
//     /// let path_list = "/usr:/dev:/usr/lib:/usr/bin:/bin";
//     /// let pathbuf_list: Vec<_> = path_list
//     ///     .split(":")
//     ///     .map(|p| PathBuf::from(p))
//     ///     .collect();
//     /// assert_eq!(script.canonicalize(&pathbuf_list).unwrap(), PathBuf::from("/bin/gzip"));
//     /// ```
//     pub fn canonicalize(&self, pathlist: &[PathBuf]) -> Result<PathBuf, Error> {
//         // if script is relative, find the path where is it located
//         if self.path.is_relative() {
//             // find the first one where script is located and build the whole path + script name
//             for path in pathlist {
//                 let mut full_path = PathBuf::new();
//                 full_path.push(path);
//                 full_path.push(&self.path);

//                 if full_path.is_file() {
//                     return full_path.canonicalize();
//                 }
//             }
//         }

//         // just check if script exists
//         if self.path.is_file() {
//             self.path.canonicalize()
//         } else {
//             Err(Error::new(ErrorKind::NotFound, "script not found"))
//         }
//     }

//     /// Replace, for each argument, the capture groups values.
//     ///
//     /// # Example
//     ///
//     /// ```rust
//     /// use std::path::PathBuf;
//     /// use regex::{Captures, Regex};
//     /// use rclf::config::Script;
//     ///
//     /// let script = Script {
//     ///     path: PathBuf::from("gzip"),
//     ///     args: Some(vec!["address=$hw".to_string(), "id=$id".to_string(), "ok".to_string()]),
//     ///     timeout: 0
//     /// };
//     /// let line = ">>> wlan0: authenticate with FF:FA:FB:FC:FD:FE";
//     /// let re = Regex::new(r"(?P<id>\w+): authenticate with (?P<hw>[A-Z:]+)").unwrap();
//     /// let caps = re.captures(line).unwrap();
//     /// let replaced = script.replace_args(caps);
//     /// assert!(replaced.is_some());
//     /// assert_eq!(replaced.unwrap(), &["address=FF:FA:FB:FC:FD:FE", "id=wlan0", "ok"]);
//     /// ```
//     pub fn replace_args<'t>(&self, caps: Captures<'t>) -> Option<Vec<String>> {
//         // if we got captures, for each argument, replace by capture groups
//         if caps.len() > 1 && self.args.is_some() {
//             // this vector will receive new arguments
//             let mut new_args = Vec::new();
//             let mut buffer = String::with_capacity(256);

//             // replace capture groups for each arg
//             for arg in self.args.as_ref().unwrap() {
//                 // replace strings like $name by capture groups values
//                 caps.expand(arg, &mut buffer);

//                 // add replaced string
//                 new_args.push(buffer.clone());

//                 // reset buffer
//                 buffer.clear();
//             }
//             return Some(new_args);
//         }
//         None
//     }

//     /// Spawns the script, and wait at most `timeout` seconds for the job to finish.
//     pub fn spawn(&self, duration: u64) -> thread::JoinHandle<()> {
//         let mut cmd = Command::new(&self.path);
//         let mut child = cmd
//             .args(&self.args.as_ref().unwrap()[..])
//             .spawn()
//             .expect("failed to execute");

//         let handle = thread::spawn(move || {
//             let one_sec = std::time::Duration::from_secs(duration);
//             let _status_code = match child.wait_timeout(one_sec).unwrap() {
//                 Some(status) => status.code(),
//                 None => {
//                     // child hasn't exited yet
//                     child.kill().unwrap();
//                     child.wait().unwrap().code()
//                 }
//             };
//         });
//         handle
//     }
// }

#[derive(Debug, Deserialize, Default)]
#[serde(from = "String")]
/// A list of options which are specific to a search.
pub struct SearchOptions {
    /// If `true`, the defined script will be run a first match.
    pub runscript: bool,

    /// If `true`, the matching line will be saved in an output file.
    pub keep_output: bool,

    /// If `true`, the logfile will be search from the beginning, regardless of any saved offset.
    pub rewind: bool,

    // a number which denotes how many lines have to match a pattern until they are considered a critical error
    pub criticalthreshold: u16,

    // is used to change this UNKNOWN to a different status. With logfilemissing=critical you can have check_file_existence-functionality
    pub logfilemissing: Option<String>,

    // controls whether the matching lines are written to a protocol file for later investigation
    pub protocol: bool,

    // controls whether the hit counter will be saved between the runs.
    // If yes, hit numbers are added until a threshold is reached (criticalthreshold).
    // Otherwise the run begins with reset counters
    pub savethresholdcount: bool,

    // controls whether an error is propagated through successive runs of check_logfiles.
    // Once an error was found, the exitcode will be non-zero until an okpattern resets it or until
    // the error expires after <second> seconds. Do not use this option until you know exactly what you do
    pub sticky: u16,

    // a number which denotes how many lines have to match a pattern until they are considered a warning
    pub warningthreshold: u16,    
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

impl From<String> for SearchOptions {
    fn from(option_list: String) -> Self {
        // create a default options structure
        let mut opt = SearchOptions::default();

        // convert the input list to a vector
        let v: Vec<_> = option_list.split(",").map(|x| x.trim()).collect();

        // use Rust macro to add bool options if any
        add_bool_option!(
            v,
            opt,
            runscript,
            rewind,
            keep_output
        );

        // other options like key=value if any
        // first build a vector of such options. We first search for = and then split according to '='
        let kv_options: Vec<_> = v.iter().filter(|&x| x.contains("=")).collect();

        // need to test whether we found key=value options
        if !kv_options.is_empty() {
            // this hash will hold key values options
            //let kvh_options: HashMap<String, String> = HashMap::new();

            // now we can safely split
            for kv in &kv_options {
                let splitted_options: Vec<_> = kv.split("=").map(|x| x.trim()).collect();
                let key = splitted_options[0];
                let value = splitted_options[1];

                // add additional non-boolean options if any
                add_typed_option!(splitted_options, criticalthreshold, opt, u16);
                add_typed_option!(splitted_options, sticky, opt, u16);
                add_typed_option!(splitted_options, warningthreshold, opt, u16);

                // special case for this
                if key == "logfilemissing" {
                    opt.logfilemissing = Some(value.to_string());
                }
            }
        }

        opt        

    }
}

/// A `enum` matching either a logfile name if only a single logfile is defined, or a list
/// of logfile names is case of command is given. This command is expected to return to the
/// the standard output the list of files to check. One of the enum variant is loaded from
/// the YAML configuration file.
#[derive(Debug, Deserialize)]
pub enum LogSource {
    #[serde(rename = "logfile")]
    LogFile(String),

    #[serde(rename = "loglist")]
    LogList(String),
}

impl LogSource {
    /// Depending on the logfile
    pub fn get_files(&self) -> Result<Vec<String>, AppError> {
        match self {
            LogSource::LogFile(s) => Ok(vec![s.clone()]),
            LogSource::LogList(cmd) => {
                let filelist = Command::new(cmd).output()?;
                let output = String::from_utf8_lossy(&filelist.stdout);
                let v: Vec<_> = output.split('\n').map(|x| x.to_string()).collect();
                Ok(v)
            }
        }
    }
}

/// This is the core structure which handles data used to search into the logfile. These are
/// gathered and refered to a tag name.
#[derive(Debug, Deserialize)]
pub struct Tag {
    /// A name to identify the tag.
    pub name: String,

    /// A list of options specific to this search. As such options are optional, add a default `serde`
    /// directive.
    #[serde(default)]
    options: SearchOptions,

    /// Script details like path, name, parameters, delay etc to be possibly run for a match.
    script: Option<Cmd>,

    /// Patterns to be checked against.
    patterns: PatternSet,
}

impl Tag {
    /// Returns the regex involved in a match, if any.
    pub fn is_match(&self, text: &str) -> Option<(PatternType, &Regex)> {
        self.patterns.is_match(text)
    }

    /// Calls the external script, by providing arguments, environment variables and path which will be searched for command.
    pub fn call_script(
        &self,
        path: Option<&str>,
        vars: &Vars,
    ) -> Result<Option<thread::JoinHandle<()>>, AppError> {
        // spawns external script if existing
        if let Some(script) = &self.script {
            match script.spawn(path, vars) {
                Ok(handle) => return Ok(Some(handle)),
                Err(e) => return Err(e),
            }
        }
        Ok(None)
    }
}

/// This is the structure mapping exactly data coming from the configuration YAML file.
#[derive(Debug, Deserialize)]
pub struct Search {
    /// the logfile name to check
    pub logfile: PathBuf,

    /// a unique identifier for this search
    pub tags: Vec<Tag>,
}

#[derive(Debug, Deserialize)]
/// A list of global options, which apply globally for all searches.
pub struct GlobalOptions {
    /// A list of paths, separated by either ':' for unix, or ';' for Windows. This is
    /// where the script, if any, will be searched for. Default to PATH or Path depending on the platform.
    #[serde(default = "self::GlobalOptions::get_path")]
    pub path: String,

    /// A directory where matches lines will be stored.
    #[serde(default = "std::env::temp_dir")]
    pub outputdir: PathBuf,

    /// A directory where the snapshot file is kept.
    #[serde(default = "std::env::temp_dir")]
    pub snapshotdir: PathBuf,
}

impl GlobalOptions {
    /// Returns the Unix PATH variable.    
    #[cfg(target_family = "unix")]
    pub fn get_path() -> String {
        std::env::var("PATH").unwrap()
    }

    /// Returns the Windows Path variable.
    #[cfg(target_family = "windows")]
    fn get_path() -> String {
        std::env::var("Path").unwrap()
    }

    /// Builds a default Some(GlobalOptions) to handle cases where the `global:` tag is not present.
    fn default() -> Option<Self> {
        Some(
            GlobalOptions {
                path: Self::get_path(),
                outputdir: std::env::temp_dir(),
                snapshotdir: std::env::temp_dir(),
            }
        )
    }
}

/// The main search configuration used to search patterns in a logfile. This is loaded from
/// the YAML file found in the command line argument. This configuration can include a list
/// of logfiles to lookup and for each logfile, a list of regexes to match.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// List of global options, which apply for all searches.
    #[serde(default = "self::GlobalOptions::default")]
    pub global: Option<GlobalOptions>,

    /// list of searches.
    pub searches: Vec<Search>,
}

impl Config {
    /// Loads a YAML configuration string as a `Config` struct.
    pub fn from_str(s: &str) -> Result<Config, AppError> {
        // load YAML data
        let yaml = serde_yaml::from_str(s)?;
        Ok(yaml)
    }

    /// Loads a YAML configuration file as a `Config` struct.
    pub fn from_file<P: AsRef<Path>>(file_name: P) -> Result<Config, AppError> {
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     //use std::path::PathBuf;

//     #[test]
//     fn test_canonicalize() {
//         use std::io::ErrorKind;
//         use std::path::PathBuf;

//         let script = Script {
//             path: PathBuf::from("foo"),
//             args: None,
//             timeout: 0,
//         };
//         let path_list = "/usr:/dev:/usr/lib:/usr/bin:/bin";
//         let pathbuf_list: Vec<_> = path_list.split(":").map(|p| PathBuf::from(p)).collect();
//         assert_eq!(
//             script.canonicalize(&pathbuf_list).unwrap_err().kind(),
//             ErrorKind::NotFound
//         );
//     }
// }
