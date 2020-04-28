use std::convert::From;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Read};
use std::path::{Path, PathBuf};

use regex::RegexSet;
use serde::{Deserialize, Serialize};

use crate::error::*;
use crate::pattern::{Pattern, PatternSet};

#[cfg(target_os = "linux")]
const SEPARATOR: char = ':';

#[cfg(target_os = "windows")]
const SEPARATOR: char = ';';

#[derive(Debug, Deserialize)]
#[serde(from = "String")]
pub struct PathList(Vec<PathBuf>);

impl From<String> for PathList {
    fn from(list: String) -> Self {
        PathList(list.split(SEPARATOR).map(|p| PathBuf::from(p)).collect())
    }
}

/// A helper structure to represent a script or command to be run on each match.
#[derive(Debug, Deserialize)]
pub struct Script {
    // name of the script to spawn without path
    pub name: PathBuf,

    // list of its optional paths
    //pub pathlist: Option<String>,

    // list of its optional arguments
    pub args: Option<Vec<String>>,
}

impl Script {
    /// Returns the canonical, absolute form of the path with all intermediate
    /// components normalized and symbolic links resolved.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use clf::config::Script;
    ///
    /// let script = Script {
    ///     name: PathBuf::from("gzip"),
    ///     args: None
    /// };
    /// let path_list = "/usr:/dev:/usr/lib:/usr/bin:/bin";
    /// let pathbuf_list: Vec<_> = path_list
    ///     .split(":")
    ///     .map(|p| PathBuf::from(p))
    ///     .collect();
    /// assert_eq!(script.canonicalize(&pathbuf_list).unwrap(), PathBuf::from("/bin/gzip"));
    /// ```
    pub fn canonicalize(&self, pathlist: &[PathBuf]) -> Result<PathBuf, Error> {
        // if script is relative, find the path where is it located
        if self.name.is_relative() {
            // at least, if script is relative, we need to find it in at least one
            // path from pathlist. So, in this case, pathlist must exist
            // if pathlist.is_none() {
            //     return Err(Error::new(ErrorKind::NotFound, "script not found"));
            // }

            // // path separator is OS dependant
            // let path_sep = if cfg!(unix) {
            //     ":"
            // } else if cfg!(windows) {
            //     ";"
            // } else {
            //     unimplemented!("OS is not supported")
            // };

            // split the string to get individual paths
            //let path_vec: Vec<_> = pathlist.as_ref().unwrap().split(path_sep).collect();

            // find the first one where script is located and build the whole path + script name
            for path in pathlist {
                let mut full_path = PathBuf::new();
                full_path.push(path);
                full_path.push(&self.name);

                if full_path.is_file() {
                    return full_path.canonicalize();
                }
            }
        }

        // just check if script exists
        if self.name.is_file() {
            self.name.canonicalize()
        } else {
            Err(Error::new(ErrorKind::NotFound, "script not found"))
        }
    }

    // pub fn exec
    // pub fn replace_args
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct SearchOptions {
    /// if `true`, the defined script will be run a first match
    pub runscript: bool,

    /// if `true`, the matching line will be saved in an output file
    pub keep_output: bool,

    /// if `true`, the logfile will be search from the beginning, regardless of any saved offset
    pub rewind: bool,
}

#[derive(Debug, Deserialize)]
#[allow(non_camel_case_types)]
pub enum LogSource {
    logfile(String),
    loglist(String),
}

#[derive(Debug, Deserialize)]
pub struct Search {
    /// a unique identifier for this search
    pub tag: String,

    /// the logfile name to check
    pub logfile: PathBuf,

    /// a list of options specific to this search. As such options are optional, add a default serde
    /// directive
    #[serde(default)]
    pub options: SearchOptions,

    /// a script details like path, name, parameters, delay etc to be possibly run for a match
    pub script: Option<Script>,

    /// patterns to be checked against
    pub patterns: PatternSet,
}

#[derive(Debug, Deserialize)]
pub struct Global {
    // A list of paths, separated by either ':' for unix, or ';' for windows. This is
    // where the script, if any, will be searched for.
    pathlist: Option<PathList>,

    // A directory where matches lines will be stored.
    #[serde(default = "std::env::temp_dir")]
    outputdir: PathBuf,

    // A directory where the snapshot file is kept.
    #[serde(default = "std::env::temp_dir")]
    snapshotdir: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub global: Global,
    pub searches: Vec<Search>,
}

impl Config {
    /// Loads a configuration file as a Config struct.
    pub fn from_str(s: &str) -> Result<Config, AppError> {
        // load YAML data
        let yaml = serde_yaml::from_str(s)?;
        Ok(yaml)
    }

    /// Loads a configuration file as a Config struct.
    pub fn from_file(file_name: &str) -> Result<Config, AppError> {
        // open YAML file
        let file = File::open(file_name)?;

        // load YAML data
        let yaml = serde_yaml::from_reader(file)?;
        Ok(yaml)
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, Script};
    //use std::path::PathBuf;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_load() {
        let toml = r#"
global:
    pathlist: "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

searches:
    - tag: "tag1"
      logfile: "/var/log/syslog"
      options:
            runscript: true
      script:
            name: /tmp/my_script.sh,
            args: ['arg1', 'arg2', 'arg3']
      patterns:
            critical:
                regexes: ["^ERROR", "FATAL", "PANIC"]
                exceptions: ["^SLIGHT_ERROR", "WARNING", "NOT IMPORTANT$"]

            warning:
                regexes: ["^ERROR", "FATAL", "PANIC"]
                exceptions: ["^SLIGHT_ERROR", "WARNING", "NOT IMPORTANT$"]
        
            ok:
                regexes: ["^ERROR", "FATAL", "PANIC"]

    - tag: "tag2"
      logfile: "/var/log/syslog"
      script:
            name: /tmp/my_script.sh,
            args: ['arg1', 'arg2', 'arg3']
      patterns:
            critical:
                regexes: ["^ERROR", "FATAL", "PANIC"]
                exceptions: ["^SLIGHT_ERROR", "WARNING", "NOT IMPORTANT$"]
"#;

        let config = Config::from_str(toml).unwrap();

        // test global struct
        assert_eq!(config.global.pathlist.unwrap().0.len(), 6);
        //assert_eq!(config.global.snapshotdir, PathBuf::from("/tmp"));

        assert_eq!(config.searches.len(), 2);
    }

    #[test]
    fn test_canonicalize() {
        use std::io::ErrorKind;
        use std::path::PathBuf;

        let script = Script {
            name: PathBuf::from("foo"),
            args: None,
        };
        let path_list = "/usr:/dev:/usr/lib:/usr/bin:/bin";
        let pathbuf_list: Vec<_> = path_list.split(":").map(|p| PathBuf::from(p)).collect();
        assert_eq!(
            script.canonicalize(&pathbuf_list).unwrap_err().kind(),
            ErrorKind::NotFound
        );
    }
}
