//! List of Nagios specific const or structures like errors, exit functions or codes, etc.
use std::fmt;
use std::str::FromStr;

use serde::Deserialize;

use crate::logfile::rundata::RunData;

/// Helper macro to define all Nagios exit functions.
macro_rules! create_exit {
    ($name:ident, $code:expr) => {
        #[allow(dead_code)]
        pub fn $name(msg: &str) {
            Nagios::exit(msg, $code);
        }
    };
}
/// Nagios exit functions.
pub struct Nagios;

impl Nagios {
    #[inline(always)]
    pub fn exit(msg: &str, code: NagiosError) {
        println!("{}: {}", String::from(&code), msg);
        std::process::exit(code as i32);
    }

    create_exit!(exit_ok, NagiosError::OK);
    create_exit!(exit_warning, NagiosError::WARNING);
    create_exit!(exit_critical, NagiosError::CRITICAL);
    create_exit!(exit_unknown, NagiosError::UNKNOWN);

    #[inline(always)]
    pub fn exit_with(code: NagiosError) {
        std::process::exit(code as i32);
    }
}

/// Enum list of Nagios error codes.
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all(deserialize = "lowercase"))]
pub enum NagiosError {
    OK = 0,
    WARNING = 1,
    CRITICAL = 2,
    UNKNOWN = 3,
}

/// Default implementation which boils down to unknown
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

/// Nagios protocol version.
#[derive(Debug)]
pub enum NagiosVersion {
    Nrpe2,
    Nrpe3,
}

/// Used from cli options.
impl FromStr for NagiosVersion {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "2" => Ok(NagiosVersion::Nrpe2),
            "3" => Ok(NagiosVersion::Nrpe3),
            _ => Err("unknow Nagios version"),
        }
    }
}

/// This will count critical & warning errors, and reported as the plugin output.
/// Or en IO error when dealing with the logfile.
#[derive(Debug, Default)]
pub struct NagiosExit {
    /// Number of matches triggered by a critical pattern.
    pub critical_count: u64,

    /// Number of matches triggered by a warning pattern.
    pub warning_count: u64,

    /// Number of unknowns due to errors reading logfiles.
    pub unknown_count: u64,

    /// Optional error if an error occured reading file
    pub error_msg: Option<String>,
}

impl From<&RunData> for NagiosExit {
    fn from(run_data: &RunData) -> Self {
        let mut nagios_exit = NagiosExit::default();

        nagios_exit.critical_count = run_data.counters.critical_count;
        nagios_exit.warning_count = run_data.counters.warning_count;
        if run_data.last_error.is_some() {
            nagios_exit.unknown_count = 1;
            let error_msg = format!("{}", run_data.last_error.as_ref().unwrap());
            nagios_exit.error_msg = Some(error_msg);
        } else {
            nagios_exit.error_msg = None;
        }
        nagios_exit
    }
}

/// Get the exit code from the NagiosExit
impl From<&NagiosExit> for NagiosError {
    fn from(m: &NagiosExit) -> Self {
        match m {
            // neither errors nor warnings
            NagiosExit {
                critical_count: 0,
                warning_count: 0,
                unknown_count: 0,
                error_msg: _,
            } => NagiosError::OK,

            // unkowns only
            NagiosExit {
                critical_count: 0,
                warning_count: 0,
                unknown_count: _,
                error_msg: _,
            } => NagiosError::UNKNOWN,

            // only warnings errors
            NagiosExit {
                critical_count: 0,
                warning_count: _,
                unknown_count: _,
                error_msg: _,
            } => NagiosError::WARNING,

            // critical errors
            NagiosExit {
                critical_count: _,
                warning_count: _,
                unknown_count: _,
                error_msg: _,
            } => NagiosError::CRITICAL,
        }
    }
}

/// Formatted string used to output to NRPE
impl fmt::Display for NagiosExit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // get error code from counters
        let nagios_err = NagiosError::from(self);

        // output is depending whether we found an error
        if self.error_msg.is_none() {
            write!(
                f,
                "{:?}: (errors:{}, warnings:{}, unknowns:{})",
                nagios_err, self.critical_count, self.warning_count, self.unknown_count
            )
        } else {
            write!(
                f,
                "{:?}: (errors:{}, warnings:{}, unknowns:{}) - error: {}",
                nagios_err,
                self.critical_count,
                self.warning_count,
                self.unknown_count,
                self.error_msg.as_ref().unwrap()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::misc::error::{AppCustomErrorKind, AppError};

    #[test]
    fn display() {
        let mut m = NagiosExit {
            critical_count: 10,
            warning_count: 100,
            unknown_count: 0,
            error_msg: None,
        };
        assert_eq!(
            &format!("{}", m),
            "CRITICAL: (errors:10, warnings:100, unknowns:0)"
        );

        m.unknown_count = 1;
        assert_eq!(
            &format!("{}", m),
            "CRITICAL: (errors:10, warnings:100, unknowns:1)"
        );
    }

    #[test]
    fn from() {
        let mut m = NagiosExit {
            critical_count: 0,
            warning_count: 0,
            unknown_count: 0,
            error_msg: None,
        };
        assert_eq!(NagiosError::from(&m), NagiosError::OK);

        m.warning_count = 1;
        assert_eq!(NagiosError::from(&m), NagiosError::WARNING);

        m.unknown_count = 1;
        assert_eq!(NagiosError::from(&m), NagiosError::WARNING);
    }

    #[test]
    fn from_str() {
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
    fn from_rundata() {
        let mut s = RunData::default();
        s.counters.critical_count = 5;
        s.counters.warning_count = 6;
        s.last_error = None;

        let mut nexit = NagiosExit::from(&s);
        assert_eq!(nexit.critical_count, 5);
        assert_eq!(nexit.warning_count, 6);
        assert!(nexit.error_msg.is_none());

        s.last_error = Some(AppError::new_custom(
            AppCustomErrorKind::SeekPosBeyondEof,
            &format!("tried to set offset beyond EOF, at offset 1000",),
        ));
        nexit = NagiosExit::from(&s);
        assert_eq!(nexit.unknown_count, 1);
        assert!(nexit.error_msg.is_some());
    }
}
