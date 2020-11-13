//! List of Nagios specific const or structures.
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use serde::Deserialize;

use crate::misc::{error::AppError, util::Cons};

// Helper macro to define all Nagios exit functions
macro_rules! create_exit {
    ($name:ident, $code:expr) => {
        #[allow(dead_code)]
        pub fn $name(msg: &str) {
            Nagios::_exit(msg, $code);
        }
    };
}
/// Nagios exit functions
pub struct Nagios;

impl Nagios {
    #[inline(always)]
    fn _exit(msg: &str, code: NagiosError) {
        println!("{} - {}", String::from(&code), msg);
        std::process::exit(code as i32);
    }

    create_exit!(exit_ok, NagiosError::OK);
    create_exit!(exit_warning, NagiosError::WARNING);
    create_exit!(exit_critical, NagiosError::CRITICAL);
    create_exit!(exit_unknown, NagiosError::UNKNOWN);

    #[inline(always)]
    pub fn exit(code: NagiosError) {
        std::process::exit(code as i32);
    }
}

/// Enum list of Nagios error codes.
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum NagiosError {
    OK = 0,
    WARNING = 1,
    CRITICAL = 2,
    UNKNOWN = 3,
}

/// Default implementation which boils down to critical
impl Default for NagiosError {
    fn default() -> Self {
        NagiosError::UNKNOWN
    }
}

/// Simple conversion from a string
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
impl From<&NagiosError> for String {
    fn from(e: &NagiosError) -> Self {
        match e {
            NagiosError::OK => "OK".to_string(),
            NagiosError::WARNING => "WARNING".to_string(),
            NagiosError::CRITICAL => "CRITICAL".to_string(),
            NagiosError::UNKNOWN => "UNKNOWN".to_string(),
        }
    }
}

/// Nagios protocol version
#[derive(Debug)]
pub enum NagiosVersion {
    Nrpe2,
    Nrpe3,
}

/// Used from cli options.
impl FromStr for NagiosVersion {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "2" => Ok(NagiosVersion::Nrpe2),
            "3" => Ok(NagiosVersion::Nrpe3),
            _ => panic!("unknow Nagios NRPE protocol version"),
        }
    }
}

/// This will count critical & warning errors, and reported as the plugin output.
/// Or en IO error when dealing with the logfile
#[derive(Debug, Default)]
pub struct HitCounter {
    /// Number of matches triggered by a critical pattern.
    pub critical_count: u16,

    /// Number of matches triggered by a warning pattern.
    pub warning_count: u16,

    /// Number of unknowns due to errors reading logfiles.
    pub unknown_count: u16,

    /// Optional error if an error occured reading file
    pub logfile_error: Option<AppError>,
}

/// Get the exit code from the HitCounter
impl From<&HitCounter> for NagiosError {
    fn from(m: &HitCounter) -> Self {
        match m {
            // neither errors nor warnings
            HitCounter {
                critical_count: 0,
                warning_count: 0,
                unknown_count: 0,
                logfile_error: _,
            } => NagiosError::OK,

            // unkowns only
            HitCounter {
                critical_count: 0,
                warning_count: 0,
                unknown_count: _,
                logfile_error: _,
            } => NagiosError::UNKNOWN,

            // only warnings errors
            HitCounter {
                critical_count: 0,
                warning_count: _,
                unknown_count: _,
                logfile_error: _,
            } => NagiosError::WARNING,

            // critical errors
            HitCounter {
                critical_count: _,
                warning_count: _,
                unknown_count: _,
                logfile_error: _,
            } => NagiosError::CRITICAL,
        }
    }
}

/// Formatted string used to output to NRPE
impl fmt::Display for HitCounter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // get error code from counters
        let nagios_err = NagiosError::from(self);

        // output is similar for all errors
        write!(
            f,
            "{:?} - (errors:{}, warnings:{}, unknowns:{})",
            nagios_err, self.critical_count, self.warning_count, self.unknown_count
        )
    }
}

/// This will hold error counters for each logfile processed.
#[derive(Debug)]
pub struct LogfileHitCounter(HashMap<PathBuf, HitCounter>);

impl Default for LogfileHitCounter {
    fn default() -> Self {
        LogfileHitCounter(HashMap::with_capacity(Cons::DEFAULT_CONTAINER_CAPACITY))
    }
}

impl LogfileHitCounter {
    /// If calling this method, we know we're using only the enum `Stats` branch.
    pub fn or_default(&mut self, path: &PathBuf) -> &mut HitCounter {
        self.0.entry(path.clone()).or_default()
    }

    /// Calculates the global counter by summation of all counters
    pub fn global(&self) -> HitCounter {
        let mut global = HitCounter::default();

        self.0.iter().for_each(|(_, x)| {
            global.critical_count += x.critical_count;
            global.warning_count += x.warning_count;
            global.unknown_count += x.unknown_count;
        });

        global
    }

    /// A fast way to iterate through internal field.
    #[cfg(test)]
    pub fn iter(&self) -> std::collections::hash_map::Iter<PathBuf, HitCounter> {
        self.0.iter()
    }

    /// Checks whether the underlying hashmap contains an error
    #[cfg(test)]
    pub fn is_error(&self) -> bool {
        self.0.iter().any(|(_, v)| v.logfile_error.is_some())
    }
}

/// Formatted string used for plugin output
impl fmt::Display for LogfileHitCounter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = String::with_capacity(Cons::DEFAULT_STRING_CAPACITY);

        for (path, counter) in self.0.iter() {
            match &counter.logfile_error {
                None => s.push_str(&format!("{}: {}\n", path.display(), counter)),
                Some(error) => s.push_str(&format!("{}: {}\n", path.display(), error)),
            }
        }

        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    //use std::path::PathBuf;
    use super::*;
    use crate::misc::error::AppError;

    #[test]
    fn display() {
        let mut m = HitCounter {
            critical_count: 10,
            warning_count: 100,
            unknown_count: 0,
            logfile_error: None,
        };
        assert_eq!(
            &format!("{}", m),
            "CRITICAL - (errors:10, warnings:100, unknowns:0)"
        );

        m.unknown_count = 1;
        assert_eq!(
            &format!("{}", m),
            "CRITICAL - (errors:10, warnings:100, unknowns:1)"
        );
    }

    #[test]
    fn from_matcher() {
        let mut m = HitCounter {
            critical_count: 0,
            warning_count: 0,
            unknown_count: 0,
            logfile_error: None,
        };
        assert_eq!(NagiosError::from(&m), NagiosError::OK);

        m.warning_count = 1;
        assert_eq!(NagiosError::from(&m), NagiosError::WARNING);

        m.unknown_count = 1;
        assert_eq!(NagiosError::from(&m), NagiosError::WARNING);
    }

    #[test]
    fn convert() {
        let mut err = NagiosError::from_str("ok").unwrap();
        assert_eq!(err, NagiosError::OK);

        err = NagiosError::from_str("CRITICAL").unwrap();
        assert_eq!(err, NagiosError::CRITICAL);

        err = NagiosError::from_str("warning").unwrap();
        assert_eq!(err, NagiosError::WARNING);

        err = NagiosError::from_str("foo").unwrap();
        assert_eq!(err, NagiosError::UNKNOWN);
    }

    #[test]
    fn logfile_matcher() {
        let mut m = LogfileHitCounter::default();
        let mut a = m.or_default(&PathBuf::from("/usr/bin/gzip"));
        a.logfile_error = Some(AppError::new(
            crate::misc::error::AppCustomErrorKind::UnsupportedPatternType,
            "foo",
        ));
        let _b = m.or_default(&PathBuf::from("/usr/bin/md5sum"));

        assert_eq!(m.iter().count(), 2);
        assert!(m.is_error());
    }
}
