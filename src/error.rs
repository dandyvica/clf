//! All structures involved in error management. It combines a list a Rust standard library
//! error types, used crates error types and a specific one to the application.
use std::{fmt, io, num};

// a macro helper to return custom app errors
// #[macro_export]
// macro_rules! app_err {
//     ($err:expr, $x:expr) => {
//         Err(AppError::App {
//             err: $err.0,
//             msg: $err.1.replace("{}", $x),
//         })
//     };
// }

// list here all error messages
// pub type ErrorBundle = (AppCustomError, &'static str);
// pub const MSG001: ErrorBundle = (AppCustomError::FileHasNoRoot, "file <{}> has no root");
// pub const MSG002: ErrorBundle = (
//     AppCustomError::FileNotAccessible,
//     "file <{}> is not accessible",
// );
// pub const MSG003: ErrorBundle = (AppCustomError::NotAFile, "file <{}> is not a file");

/// Error kind specific to an application error, different from standard errors.
#[derive(Debug, PartialEq)]
pub enum AppCustomErrorKind {
    SeekPosBeyondEof,
    NoPathForScript,
    UnsupportedPatternType,
    FileNotUsable,
}

/// A specific error type combining all error types.
#[derive(Debug)]
pub enum AppError {
    Io(io::Error),
    Regex(regex::Error),
    Parse(num::ParseIntError),
    Yaml(serde_yaml::Error),
    Json(serde_json::Error),
    SystemTime(std::time::SystemTimeError),
    App {
        err: AppCustomErrorKind,
        msg: String,
    },
}

// impl Error for AppError {
//     fn description(&self) -> &str {
//         match *self {
//             AppError::Io(ref err) => err.description(),
//             AppError::Regex(ref err) => err.description(),
//             AppError::Parse(ref err) => err.description(),
//             AppError::JSON(ref err) => err.description(),
//         }
//     }
// }

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AppError::Io(ref err) => err.fmt(f),
            AppError::Regex(ref err) => err.fmt(f),
            AppError::Parse(ref err) => err.fmt(f),
            AppError::Yaml(ref err) => err.fmt(f),
            AppError::Json(ref err) => err.fmt(f),
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
