//! Contains the configuration of the name of a logfile: it could be either a single file, or a command giving the list of files.
use std::fmt::Display;
use std::path::PathBuf;

use serde::Deserialize;

/// A `enum` matching either a logfile name if only a single logfile is defined, or a list
/// of logfile names is case of command is given. This command is expected to return to the
/// the standard output the list of files to check. One of the enum variant is loaded from
/// the YAML configuration file.
#[derive(Debug, Deserialize, Clone)]
pub enum LogSource {
    #[serde(rename = "path")]
    LogFile(PathBuf),

    #[serde(rename = "list")]
    LogList(Vec<String>),
}

impl LogSource {
    pub const fn is_path(&self) -> bool {
        matches!(*self, LogSource::LogFile(_))
    }
}

impl Display for LogSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogSource::LogFile(logfile) => write!(f, "{}", logfile.display()),
            LogSource::LogList(_) => unimplemented!("LogSource::LogList not permitted !"),
        }
    }
}

impl Default for LogSource {
    fn default() -> Self {
        LogSource::LogFile(PathBuf::from(""))
    }
}
