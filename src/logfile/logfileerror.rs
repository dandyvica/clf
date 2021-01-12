//! Collect all immediate logfile errors i.e. those related to opening or metadata.
use std::collections::{hash_map::Iter, HashMap};
use std::ops::Deref;
use std::path::PathBuf;

use crate::misc::{error::AppError, nagios::NagiosError};

#[derive(Debug)]
/// A structure to hold possible logfile errors and related Nagios errors.
pub struct LogFileAccessError {
    pub nagios_error: NagiosError,
    pub error: AppError,
}

/// A list of logfile errors.
pub struct LogFileAccessErrorList(HashMap<PathBuf, LogFileAccessError>);

impl Default for LogFileAccessErrorList {
    fn default() -> Self {
        LogFileAccessErrorList(HashMap::new())
    }
}

impl LogFileAccessErrorList {
    pub fn set_error(&mut self, path: &PathBuf, error: AppError, nagios_error: &NagiosError) {
        let logfile_error = LogFileAccessError {
            nagios_error: nagios_error.clone(),
            error,
        };
        self.0.insert(path.clone(), logfile_error);
    }

    pub fn iter(&self) -> Iter<'_, PathBuf, LogFileAccessError> {
        self.0.iter()
    }
}

impl Deref for LogFileAccessErrorList {
    type Target = HashMap<PathBuf, LogFileAccessError>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
