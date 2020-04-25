use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Read};
use std::path::{Path, PathBuf};

use regex::RegexSet;
use serde::{Deserialize, Serialize};

use crate::error::*;
use crate::pattern::{Pattern, PatternType};

/// A helper structure to represent a script or command to be run on each match.
#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct Search {
    // a unique identifier
    pub tag: String,

    // logfile name
    pub logfile: Option<PathBuf>,

    // options specific to a search
    //pub options: Options,

    // script details like path, name, parameters, delay etc
    pub script: Option<Script>,

    // vector of patterns to look for
    pub patterns: Vec<Pattern>,
    // logfile data as name, etc
    //pub status_file: StatusFile,
}

#[derive(Debug, Deserialize)]
pub struct Global {
    pathlist: String,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    global: Global,
    searches: Search,
}

impl Config {
    /// Loads a configuration file as a Config struct.
    ///
    /// # Example
    ///
    pub fn from_str(s: &str) -> Result<Config, AppError> {
        // load YAML data
        let yaml = serde_yaml::from_str(s)?;
        Ok(yaml)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::config::{Config, Script};

    #[test]
    fn test_load() {
        let toml = r#"
global:
    pathlist: "/usr/bin"

searches:
    tag: "tag1"
    logfile: "/var/log/syslog"
    script:
        name: /tmp/my_script.sh,
        args: ['arg1', 'arg2', 'arg3']
    patterns:
        - type: critical
          regexes: ["^ERROR", "FATAL", "PANIC"]
          exceptions: ["^SLIGHT_ERROR", "WARNING", "NOT IMPORTANT$"]

        - type: warning
          regexes: ["^ERROR", "FATAL", "PANIC"]
          exceptions: ["^SLIGHT_ERROR", "WARNING", "NOT IMPORTANT$"]

        - type: ok
          regexes: ["^ERROR", "FATAL", "PANIC"]
"#;

        let config = Config::from_str(toml).unwrap();

        assert_eq!(config.global.pathlist, "/usr/bin");

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