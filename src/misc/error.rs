//! All structures involved in error management. It combines a list a Rust standard library
//! error types, used crates error types and a specific one to the application.
//! Use `map_err` method to report errors with context (see examples in tests).
use std::clone::Clone;
use std::{fmt, io, num};

/// A specific custom `Result` for all functions
pub type AppResult<T> = Result<T, AppError>;

/// Error kind specific to an application error, different from standard errors.
#[derive(Debug, PartialEq)]
pub enum AppCustomErrorKind {
    SeekPosBeyondEof,
    UnsupportedPatternType,
    FileNotUsable,
    FilePathNotAbsolute,
    UnsupportedSearchOption,
    OsStringConversionError,
    PhantomCloneError,
}

impl fmt::Display for AppCustomErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppCustomErrorKind::SeekPosBeyondEof => {
                write!(f, "a seek operation was beyond end of file")
            }
            AppCustomErrorKind::UnsupportedPatternType => {
                write!(f, "the specified type of pattern is not supported")
            }
            AppCustomErrorKind::FileNotUsable => {
                write!(f, "file is not a file, probably a directory")
            }
            AppCustomErrorKind::FilePathNotAbsolute => write!(f, "the file path is not absolute"),
            AppCustomErrorKind::UnsupportedSearchOption => write!(f, "search option not supported"),
            AppCustomErrorKind::OsStringConversionError => {
                write!(f, "conversion from OsString failed")
            }
            AppCustomErrorKind::PhantomCloneError => write!(f, "no error"),
        }
    }
}

/// A specific error type combining all possible error types in the app.
#[derive(Debug)]
pub enum InternalError {
    Io(io::Error),
    Regex(regex::Error),
    Parse(num::ParseIntError),
    Yaml(serde_yaml::Error),
    Json(serde_json::Error),
    SystemTime(std::time::SystemTimeError),
    Utf8(std::str::Utf8Error),
    Custom(AppCustomErrorKind),
}

/// To simplify definition of all error conversions.
macro_rules! from_error {
    ($e:path, $f:path) => {
        impl From<$e> for InternalError {
            fn from(err: $e) -> InternalError {
                $f(err)
            }
        }
    };
}

from_error!(io::Error, InternalError::Io);
from_error!(regex::Error, InternalError::Regex);
from_error!(serde_yaml::Error, InternalError::Yaml);
from_error!(serde_json::Error, InternalError::Json);
from_error!(std::time::SystemTimeError, InternalError::SystemTime);
from_error!(num::ParseIntError, InternalError::Parse);
from_error!(std::str::Utf8Error, InternalError::Utf8);

/// Custom error which will be used for all errors conversions and throughout the code.
#[derive(Debug)]
pub struct AppError {
    pub error_kind: InternalError,
    pub msg: String,
}

impl AppError {
    /// A simple and convenient creation of a new application error
    pub fn new_custom(kind: AppCustomErrorKind, msg: &str) -> Self {
        AppError {
            error_kind: InternalError::Custom(kind),
            msg: msg.to_string(),
        }
    }

    /// Convert from an internal error
    pub fn from_error<T: Into<InternalError>>(err: T, msg: &str) -> Self {
        AppError {
            error_kind: err.into(),
            msg: msg.to_string(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.error_kind {
            InternalError::Io(ref err) => write!(f, "I/O error: {} ({})", self.msg, err),
            InternalError::Regex(ref err) => write!(f, "regex error: {} ({})", self.msg, err),
            InternalError::Parse(ref err) => write!(f, "conversion error: {} ({})", self.msg, err),
            InternalError::Yaml(ref err) => write!(f, "YAML error: {} ({})", self.msg, err),
            InternalError::Json(ref err) => write!(f, "JSON error: {} ({})", self.msg, err),
            InternalError::Utf8(ref err) => {
                write!(f, "Utf8 conversion error: {} ({})", self.msg, err)
            }
            InternalError::SystemTime(ref err) => {
                write!(f, "system time error: {} ({})", self.msg, err)
            }
            InternalError::Custom(ref err) => write!(f, "custom error: {} ({})", self.msg, err),
        }
    }
}

impl Clone for AppError {
    fn clone(&self) -> Self {
        AppError::new_custom(
            AppCustomErrorKind::PhantomCloneError,
            &format!("fake clone error"),
        )
    }
}

/// To simplify definition of all error conversions.
#[macro_export]
macro_rules! context {
    ($err:ident, $fmt:expr, $($arg:tt)*) => {
        AppError::from_error(
            $err,
            &format!($fmt, $($arg)*)
        )

    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::time::{Duration, SystemTime};

    use regex::Regex;
    use serde::Deserialize;

    // dummy struct
    #[derive(Debug, Deserialize)]
    struct A;

    #[test]
    fn error() {
        let err = file().unwrap_err();
        assert!(matches!(err.error_kind, InternalError::Io(_)));
        println!("{}", err);

        let err = regex().unwrap_err();
        assert!(matches!(err.error_kind, InternalError::Regex(_)));
        println!("{}", err);

        let err = parse().unwrap_err();
        assert!(matches!(err.error_kind, InternalError::Parse(_)));
        println!("{}", err);

        let err = yaml().unwrap_err();
        assert!(matches!(err.error_kind, InternalError::Yaml(_)));
        println!("{}", err);

        let err = json().unwrap_err();
        assert!(matches!(err.error_kind, InternalError::Json(_)));
        println!("{}", err);

        let err = systemtime().unwrap_err();
        assert!(matches!(err.error_kind, InternalError::SystemTime(_)));
        println!("{}", err);

        let err = utf8().unwrap_err();
        assert!(matches!(err.error_kind, InternalError::Utf8(_)));
        println!("{}", err);

        let err = custom();
        assert!(matches!(err.error_kind, InternalError::Custom(_)));
        println!("{}", err);

        assert!(
            matches!(err.clone().error_kind, InternalError::Custom(x) if x == AppCustomErrorKind::PhantomCloneError)
        );
    }

    #[cfg(target_family = "unix")]
    fn file() -> AppResult<File> {
        let path = "/foo/foo.foo";
        let file = File::open(path).map_err(|e| context!(e, "unable to open file {}", path))?;
        Ok(file)
    }

    #[cfg(target_family = "windows")]
    fn file() -> AppResult<File> {
        let path = r"c:\foo\foo.foo";
        let file = File::open(path).map_err(|e| context!(e, "unable to open file {}", path))?;
        Ok(file)
    }

    fn regex() -> AppResult<Regex> {
        let s = "foo(";
        let re = Regex::new(s).map_err(|e| context!(e, "unable to create regex {}", s))?;
        Ok(re)
    }

    fn parse() -> AppResult<usize> {
        let s = "18a";
        let value = s
            .parse::<usize>()
            .map_err(|e| context!(e, "unable to convert {} to integer", s))?;
        Ok(value)
    }

    fn yaml() -> AppResult<A> {
        let s = "-foo";

        let yaml: A = serde_yaml::from_str(s)
            .map_err(|e| context!(e, "unable to load YAML string '{}'", s))?;
        Ok(yaml)
    }

    fn json() -> AppResult<A> {
        let s = "{";

        let json: A = serde_json::from_str(s)
            .map_err(|e| context!(e, "unable to load JSON string '{}'", s))?;
        Ok(json)
    }

    fn systemtime() -> AppResult<Duration> {
        let sys_time = SystemTime::now();
        let new_sys_time = SystemTime::now();

        let difference = sys_time
            .duration_since(new_sys_time)
            .map_err(|e| context!(e, "error in duration_since() call",))?;

        Ok(difference)
    }

    fn utf8() -> AppResult<String> {
        use std::str;
        // some bytes, in a vector
        let non_utf8 = vec![240, 159, 146];

        // We know these bytes are valid, so just use `unwrap()`.
        let s = str::from_utf8(&non_utf8)
            .map_err(|e| context!(e, "{:?} in not an UTF8 string", non_utf8))?;
        Ok(s.to_string())
    }

    fn custom() -> AppError {
        let path = "/foo/foo.foo";
        let custom_err = AppError::new_custom(
            AppCustomErrorKind::FileNotUsable,
            &format!("file '{}' not usable", path),
        );
        custom_err
    }
}
