//! All structures involved in error management. It combines a list a Rust standard library
//! error types, used crates error types and a specific one to the application.
use std::clone::Clone;
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
    FilePathNotAbsolute,
    UnsupportedSearchOption,
    OsStringConversionError,
    UnknowError,
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

impl Clone for AppError {
    fn clone(&self) -> Self {
        AppError::new(AppCustomErrorKind::UnknowError, "")
    }
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

/// To simplify definition of all error conversions.
macro_rules! from_error {
    ($e:path, $f:path) => {
        impl From<$e> for AppError {
            fn from(err: $e) -> AppError {
                $f(err)
            }
        }
    };
}

from_error!(io::Error, AppError::Io);
from_error!(regex::Error, AppError::Regex);
from_error!(serde_yaml::Error, AppError::Yaml);
from_error!(serde_json::Error, AppError::Json);
from_error!(std::time::SystemTimeError, AppError::SystemTime);
from_error!(num::ParseIntError, AppError::Parse);
from_error!(std::str::Utf8Error, AppError::Utf8Error);

impl From<std::ffi::OsString> for AppError {
    fn from(err: std::ffi::OsString) -> AppError {
        AppError::new(
            AppCustomErrorKind::OsStringConversionError,
            &format!("error converting: {:?}", err),
        )
    }
}
