//! List of Nagios specific const or structures.
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use crate::{config::LogfileMissing, error::AppError};

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
#[derive(Debug, Default)]
pub struct MatchCounter {
    /// Number of matches triggered by a critical pattern.
    pub critical_count: u16,

    /// Number of matches triggered by a warning pattern.
    pub warning_count: u16,

    /// Error message in case of an error on opening a logfile
    pub app_error: (LogfileMissing, Option<String>),
}

impl MatchCounter {
    /// Builds the string for plugin output
    pub fn output(&self) -> (String, i32) {
        match self {
            // neither errors nor warnings
            MatchCounter {
                critical_count: 0,
                warning_count: 0,
                app_error: (_, None),
            } => (
                format!("{} - no errors or warnings", EXIT_NAGIOS_OK.1),
                EXIT_NAGIOS_OK.0,
            ),

            // only warnings errors
            MatchCounter {
                critical_count,
                warning_count: 0,
                app_error: (_, None),
            } => (
                format!("{} - ({} errors)", EXIT_NAGIOS_CRITICAL.1, critical_count),
                EXIT_NAGIOS_CRITICAL.0,
            ),

            // only critical errors
            MatchCounter {
                critical_count: 0,
                warning_count,
                app_error: (_, None),
            } => (
                format!("{} - ({} warnings)", EXIT_NAGIOS_WARNING.1, warning_count),
                EXIT_NAGIOS_WARNING.0,
            ),

            // both errors and warnings
            MatchCounter {
                critical_count,
                warning_count,
                app_error: (_, None),
            } => (
                format!(
                    "{} - ({} errors, {} warnings)",
                    EXIT_NAGIOS_CRITICAL.1, critical_count, warning_count
                ),
                EXIT_NAGIOS_CRITICAL.0,
            ),

            // io error
            MatchCounter {
                critical_count: _,
                warning_count: _,
                app_error: (logfilemissing, Some(err)),
            } => match logfilemissing {
                LogfileMissing::critical => (format!("CRITICAL - {}", err), EXIT_NAGIOS_CRITICAL.0),
                LogfileMissing::warning => (format!("WARNING - {}", err), EXIT_NAGIOS_WARNING.0),
                LogfileMissing::unknown => (format!("UNKNOWN - {}", err), EXIT_NAGIOS_UNKNOWN.0),
            },
        }
    }
}

/// This will hold error counters for each logfile
#[derive(Debug)]
pub struct LogfileMatchCounter(pub HashMap<PathBuf, MatchCounter>);

impl LogfileMatchCounter {
    /// Just defines a new empty counter structure.
    pub fn new() -> Self {
        LogfileMatchCounter(HashMap::with_capacity(30))
    }

    /// Ensures a value is in the entry by inserting the default if empty, and returns a mutable reference to the value in the entry.
    pub fn or_default(&mut self, path: &PathBuf) -> &mut MatchCounter {
        self.0.entry(path.clone()).or_default()
    }

    /// A fast way to iterate through internal field.
    pub fn iter(&self) -> std::collections::hash_map::Iter<PathBuf, MatchCounter> {
        self.0.iter()
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
}
