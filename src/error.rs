use std::{fmt, io, num};

// a macro helper to return custom app errors
#[macro_export]
macro_rules! app_err {
    ($err:expr, $x:expr) => {
        Err(AppError::App {
            err: $err.0,
            msg: $err.1.replace("{}", $x),
        })
    };
}

// list here all error messages
pub type ErrorBundle = (AppCustomError, &'static str);
pub const MSG001: ErrorBundle = (AppCustomError::FileHasNoRoot, "file <{}> has no root");
pub const MSG002: ErrorBundle = (
    AppCustomError::FileNotAccessible,
    "file <{}> is not accessible",
);
pub const MSG003: ErrorBundle = (AppCustomError::NotAFile, "file <{}> is not a file");

// define our own custom error type
#[derive(Debug, PartialEq)]
pub enum AppCustomError {
    FileHasNoRoot,
    FileNotAccessible,
    NotAFile,
    SeekPosBeyondEof,
    NoPathForScript,
}

// define our own application error type
#[derive(Debug)]
pub enum AppError {
    Io(io::Error),
    Regex(regex::Error),
    Parse(num::ParseIntError),
    JSON(serde_json::error::Error),
    App { err: AppCustomError, msg: String },
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
            AppError::JSON(ref err) => err.fmt(f),
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

impl From<serde_json::error::Error> for AppError {
    fn from(err: serde_json::Error) -> AppError {
        AppError::JSON(err)
    }
}

impl From<num::ParseIntError> for AppError {
    fn from(err: num::ParseIntError) -> AppError {
        AppError::Parse(err)
    }
}
