//! All structures involved in error management. It combines a list a Rust standard library
//! error types, used crates error types and a specific one to the application.
use std::io::ErrorKind;
use std::{fmt, io, num};

/// Error kind specific to an application error, different from standard errors.
#[derive(Debug, PartialEq)]
pub enum AppCustomErrorKind {
    SeekPosBeyondEof,
    NoPathForScript,
    UnsupportedPatternType,
    FileNotUsable,
    NotAFile,
    UnsupportedSearchOption,
}

/// A specific error type combining all possible error types in the app.
#[derive(Debug)]
pub enum AppError {
    Io(io::Error),
    Regex(regex::Error),
    Parse(num::ParseIntError),
    Yaml(serde_yaml::Error),
    Json(serde_json::Error),
    SystemTime(std::time::SystemTimeError),
    Utf8Error(std::str::Utf8Error),
    App {
        err: AppCustomErrorKind,
        msg: String,
    },
}

impl AppError {
    /// A simple and convenient creation of a new application error
    pub fn new(err: AppCustomErrorKind, msg: &str) -> Self {
        AppError::App {
            err: err,
            msg: msg.to_string(),
        }
    }

    /// Returns the IO error kind branch if any
    pub fn get_ioerror(&self) -> Option<ErrorKind> {
        match self {
            AppError::Io(io_error) => Some(io_error.kind()),
            _ => None,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AppError::Io(ref err) => err.fmt(f),
            AppError::Regex(ref err) => err.fmt(f),
            AppError::Parse(ref err) => err.fmt(f),
            AppError::Yaml(ref err) => err.fmt(f),
            AppError::Json(ref err) => err.fmt(f),
            AppError::Utf8Error(ref err) => err.fmt(f),
            AppError::SystemTime(ref err) => err.fmt(f),
            AppError::App { ref err, ref msg } => {
                write!(f, "A custom error occurred {:?}, custom msg {:?}", err, msg)
            }
        }
    }
}

// define conversion methods for all types we might return in our methods
impl From<io::Error> for AppError {
    fn from(err: io::Error) -> AppError {
        AppError::Io(err)
    }
}

impl From<regex::Error> for AppError {
    fn from(err: regex::Error) -> AppError {
        AppError::Regex(err)
    }
}

impl From<serde_yaml::Error> for AppError {
    fn from(err: serde_yaml::Error) -> AppError {
        AppError::Yaml(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> AppError {
        AppError::Json(err)
    }
}

impl From<std::time::SystemTimeError> for AppError {
    fn from(err: std::time::SystemTimeError) -> AppError {
        AppError::SystemTime(err)
    }
}

impl From<num::ParseIntError> for AppError {
    fn from(err: num::ParseIntError) -> AppError {
        AppError::Parse(err)
    }
}

impl From<std::str::Utf8Error> for AppError {
    fn from(err: std::str::Utf8Error) -> AppError {
        AppError::Utf8Error(err)
    }
}
