//! List of Nagios specific const or structures.
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use serde::Deserialize;

//use crate::{config::LogfileMissing, error::AppError};

/// enum list of Nagios error codes.
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum NagiosError {
    OK = 0,
    WARNING = 1,
    CRITICAL = 2,
    UNKNOWN = 3,
}

/// Default implementation whic boils down to critical
impl Default for NagiosError {
    fn default() -> Self {
        NagiosError::UNKNOWN
    }
}

impl FromStr for NagiosError {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_ref() {
            "OK" => Ok(NagiosError::OK),
            "WARNING" => Ok(NagiosError::WARNING),
            "CRITICAL" => Ok(NagiosError::CRITICAL),
            "UNKNOWN" => Ok(NagiosError::UNKNOWN),
            &_ => Ok(NagiosError::UNKNOWN),
        }
    }
}

/// Conversion to a static string reference.
impl From<NagiosError> for &'static str {
    fn from(e: NagiosError) -> Self {
        match e {
            NagiosError::OK => "OK",
            NagiosError::WARNING => "WARNING",
            NagiosError::CRITICAL => "CRITICAL",
            NagiosError::UNKNOWN => "UNKNOWN",
        }
    }
}

/// Formatted string used to output to NRPE
// impl fmt::Display for NagiosError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!("{} - {}", self.into(), self as i32

//         match *self {
//             AppError::Io(ref err) => err.fmt(f),
//             AppError::Regex(ref err) => err.fmt(f),
//             AppError::Parse(ref err) => err.fmt(f),
//             AppError::Yaml(ref err) => err.fmt(f),
//             AppError::Json(ref err) => err.fmt(f),
//             AppError::Utf8Error(ref err) => err.fmt(f),
//             AppError::SystemTime(ref err) => err.fmt(f),
//             AppError::App { ref err, ref msg } => {
//                 write!(f, "A custom error occurred {:?}, custom msg {:?}", err, msg)
//             }
//         }
//     }
// }

/// List of Nagios exit codes & strings
pub const EXIT_NAGIOS_OK: (i32, &'static str) = (0, "OK");
pub const EXIT_NAGIOS_WARNING: (i32, &'static str) = (1, "WARNING");
pub const EXIT_NAGIOS_CRITICAL: (i32, &'static str) = (2, "CRITICAL");
pub const EXIT_NAGIOS_UNKNOWN: (i32, &'static str) = (3, "UNKNOWN");

/// Nagios protocol version
#[derive(Debug)]
pub enum NagiosVersion {
    NagiosNrpe2,
    NagiosNrpe3,
}

/// Used from cli options.
impl FromStr for NagiosVersion {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "2" => Ok(NagiosVersion::NagiosNrpe2),
            "3" => Ok(NagiosVersion::NagiosNrpe3),
            _ => panic!("unknow Nagios NRPE protocol version"),
        }
    }
}

/// This will count critical & warning errors, and reported as the plugin output.
/// Or en IO error when dealing with the logfile
#[derive(Debug, Default)]
pub struct MatchCounter {
    /// Number of matches triggered by a critical pattern.
    pub critical_count: u16,

    /// Number of matches triggered by a warning pattern.
    pub warning_count: u16,
}

/// Get the exit code from the MatchCounter
impl From<&MatchCounter> for NagiosError {
    fn from(m: &MatchCounter) -> Self {
        match m {
            // neither errors nor warnings
            MatchCounter {
                critical_count: 0,
                warning_count: 0,
            } => NagiosError::OK,

            // only warnings errors
            MatchCounter {
                critical_count: 0,
                warning_count: _,
            } => NagiosError::WARNING,

            // critical errors
            MatchCounter {
                critical_count: _,
                warning_count: _,
            } => NagiosError::CRITICAL,
        }
    }
}

impl MatchCounter {
    /// Builds the string for plugin output
    pub fn output(&self) -> String {
        match self {
            // neither errors nor warnings
            MatchCounter {
                critical_count: 0,
                warning_count: 0,
            } => format!("{:?} - no errors or warnings", NagiosError::OK),

            // only warnings errors
            MatchCounter {
                critical_count,
                warning_count: 0,
            } => format!("{:?} - ({} errors)", NagiosError::CRITICAL, critical_count),

            // only critical errors
            MatchCounter {
                critical_count: 0,
                warning_count,
            } => format!("{:?} - ({} warnings)", NagiosError::WARNING, warning_count),

            // both errors and warnings
            MatchCounter {
                critical_count,
                warning_count,
            } => format!(
                "{:?} - ({} errors, {} warnings)",
                NagiosError::CRITICAL,
                critical_count,
                warning_count,
            ),
        }
    }
}

/// A counter for logfiles: either a set a counter, or an error message when this logfile can't be opened.
#[derive(Debug)]
pub enum LogfileCounter {
    Stats(MatchCounter),
    ErrorMsg(String),
}

impl LogfileCounter {
    /// Helper function to increment the `Stats` enum branch.
    pub fn inc_warning(&mut self) {
        match self {
            LogfileCounter::Stats(counter) => counter.warning_count += 1,
            LogfileCounter::ErrorMsg(_) => (),
        }
    }

    /// Helper function to increment the `Stats` enum branch.
    pub fn inc_critical(&mut self) {
        match self {
            LogfileCounter::Stats(counter) => counter.critical_count += 1,
            LogfileCounter::ErrorMsg(_) => (),
        }
    }
}

/// This will hold error counters for each logfile processed.
#[derive(Debug)]
pub struct LogfileMatchCounter(pub HashMap<PathBuf, LogfileCounter>);

impl LogfileMatchCounter {
    /// Just defines a new empty counter structure.
    pub fn new() -> Self {
        LogfileMatchCounter(HashMap::with_capacity(30))
    }

    /// If calling this method, we know we're using only the enum `Stats` branch.
    pub fn or_default(&mut self, path: &PathBuf) -> &mut LogfileCounter {
        self.0
            .entry(path.clone())
            .or_insert_with(|| LogfileCounter::Stats(MatchCounter::default()))
    }

    /// A fast way to iterate through internal field.
    pub fn iter(&self) -> std::collections::hash_map::Iter<PathBuf, LogfileCounter> {
        self.0.iter()
    }

    /// Sets the error message for the `ErrorMsg` branch.
    pub fn set_error(&mut self, path: &PathBuf, msg: &str) {
        let _ = self
            .0
            .insert(path.clone(), LogfileCounter::ErrorMsg(msg.to_string()));
    }
}

mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn or_insert() {
        let mut counter = LogfileMatchCounter::new();
        assert!(counter.0.is_empty());

        let x = counter.or_default(&PathBuf::from("/usr/bin/gzip"));
        x.critical_count = 100;
        x.warning_count = 200;

        let y = counter.or_default(&PathBuf::from("/usr/bin/gzip"));
        assert_eq!(y.critical_count, 100);
        assert_eq!(y.warning_count, 200);
    }

    #[test]
    fn convert() {
        let err = NagiosError::from_str("ok").unwrap();
        assert_eq!(err, NagiosError::OK);
    }
}
