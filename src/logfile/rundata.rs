//! A structure representing all the data specific to a run.
use chrono::prelude::*;
use serde::{Deserialize, Serialize, Serializer};

use crate::misc::error::AppError;

use crate::configuration::options::SearchOptions;
use crate::configuration::pattern::{PatternCounters, PatternType};

/// A wrapper to store log file processing data.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct RunData {
    /// pid of the process currently running
    pub pid: u32,

    /// starting position of the search
    pub start_offset: u64,

    /// starting line of the search
    pub start_line: u64,

    /// position of the last run. Used to seek the file pointer to this point.
    pub last_offset: u64,

    /// last line number during the last search
    pub last_line: u64,

    /// last time logfile were processed: printable date/time
    #[serde(serialize_with = "timestamp_to_string", skip_deserializing)]
    pub last_run: f64,

    /// last time logfile were processed in seconds: used to check retention
    //#[serde(skip)]
    pub last_run_secs: u64,

    /// keep all counters here
    pub counters: PatternCounters,

    // last error when reading a logfile
    #[serde(serialize_with = "error_to_string", skip_deserializing)]
    pub last_error: Option<AppError>,
}

/// Converts the timestamp to a human readable string in the snapshot.
pub fn timestamp_to_string<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // exract integer part = number of seconds
    // frational part = number of nanoseconds
    let secs = value.trunc();
    let nanos = value.fract();
    let utc_tms = Utc.timestamp(secs as i64, (nanos * 1_000_000_000f64) as u32);
    format!("{}", utc_tms.format("%Y-%m-%d %H:%M:%S.%f")).serialize(serializer)
}

/// Converts the error to string.
pub fn error_to_string<S>(value: &Option<AppError>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if value.is_none() {
        "None".to_string().serialize(serializer)
    } else {
        format!("{}", value.as_ref().unwrap()).serialize(serializer)
    }
}

impl RunData {
    /// increment or decrement counters
    pub fn increment_counters(&mut self, pattern_type: &PatternType) {
        match pattern_type {
            PatternType::critical => self.counters.critical_count += 1,
            PatternType::warning => self.counters.warning_count += 1,
            PatternType::ok => self.counters.ok_count += 1,
        }
    }
    pub fn decrement_counters(&mut self, pattern_type: &PatternType) {
        match pattern_type {
            PatternType::critical => {
                debug_assert!(self.counters.critical_count != 0);
                self.counters.critical_count -= 1
            }
            PatternType::warning => {
                debug_assert!(self.counters.warning_count != 0);
                self.counters.warning_count -= 1
            }
            PatternType::ok => {
                debug_assert!(self.counters.ok_count != 0);
                self.counters.ok_count -= 1
            }
        }
    }

    /// Return `true` if counters reach thresholds
    pub fn is_threshold_reached(
        &mut self,
        pattern_type: &PatternType,
        options: &SearchOptions,
    ) -> bool {
        trace!(
            "pattern_type={:?}, runifok={}",
            pattern_type,
            options.runifok
        );
        // increments thresholds and compare with possible defined limits and accumulate counters for plugin output
        match pattern_type {
            PatternType::critical => {
                //self.counters.critical_count += 1;
                if self.counters.critical_count <= options.criticalthreshold {
                    return false;
                }
            }
            PatternType::warning => {
                //self.counters.warning_count += 1;
                if self.counters.warning_count <= options.warningthreshold {
                    return false;
                }
            }
            // this special Ok pattern resets counters
            PatternType::ok => {
                //self.counters.ok_count += 1;
                self.counters.critical_count = 0;
                self.counters.warning_count = 0;

                // no need to process further: don't call a script if runifok is not set
                return options.runifok;
                //return true;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_threshold_reached() {
        let mut opts = SearchOptions::default();
        let mut s = RunData::default();

        s.counters.critical_count = 5;
        s.counters.warning_count = 5;

        opts.criticalthreshold = 4;
        opts.warningthreshold = 4;
        assert!(s.is_threshold_reached(&PatternType::critical, &opts));
        //assert_eq!(s.counters.critical_count, 6);

        opts.criticalthreshold = 10;
        opts.warningthreshold = 10;
        assert!(!s.is_threshold_reached(&PatternType::warning, &opts));
        //assert_eq!(s.counters.warning_count, 6);

        opts.criticalthreshold = 1;
        opts.warningthreshold = 1;
        opts.runifok = true;
        assert!(s.is_threshold_reached(&PatternType::ok, &opts));
        //assert_eq!(s.counters.critical_count, 0);
        //assert_eq!(s.counters.warning_count, 0);
    }
}
