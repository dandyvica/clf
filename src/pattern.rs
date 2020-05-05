//! A list of structures dedicated to match text data from a logfile. It merely defines a list of
//! regexes structures, which are used to search for a pattern in a text.
//!
use std::convert::TryFrom;

use regex::{Captures, Regex, RegexSet};
use serde::Deserialize;

use crate::error::{AppCustomErrorKind, AppError};

//#[doc(hidden)]

/// A helper structure for deserializing into a `Vec<Regex>` automatically from a `Vec<String>`.
#[derive(Debug, Deserialize)]
#[serde(try_from = "Vec<String>")]
pub struct RegexVec(pub Vec<Regex>);

/// A helper structure for deserializing into a `RegexSet` automatically from a `Vec<String>`.
#[derive(Debug, Deserialize)]
#[serde(try_from = "Vec<String>")]
pub struct RegexBundle(pub RegexSet);

/// An implementation of `TryFrom` for the helper tuple struct `RegexVec`.
///
/// This just creates a `RegexVec` structure from a vector of regexes strings. This is
/// used by the `serde` deserialize process in order to automatically transforms a vector
/// of strings read from the YAML config file into a `RegexVec` structure, which contains
/// a vector of compiled `Regex` structs.
///
/// # Example
///
/// ```rust
/// use std::convert::TryFrom;
/// use clf::pattern::RegexVec;
///
/// let regs = RegexVec::try_from(vec!["^#".to_string(), ";$".to_string()]).unwrap();
/// assert_eq!(regs.0.len(), 2);
/// ```
impl TryFrom<Vec<String>> for RegexVec {
    type Error = AppError;

    fn try_from(list: Vec<String>) -> Result<Self, Self::Error> {
        let mut v: Vec<Regex> = Vec::new();
        for re in &list {
            // replace
            v.push(Regex::new(re)?);
        }
        Ok(RegexVec(v))
    }
}

/// An implementation of `TryFrom` for the help tuple struct `RegexBundle`.
///
/// This just creates a `RegexBundle` structure from a vector of regexes strings. This is
/// used by the `serde` deserialize process in order to automatically transforms a vector
/// of strings read from the YAML config file into a `RegexBundle` structure, which contains
/// a vector of compiled `RegexSet` structure.
///
/// # Example
///
/// ```rust
/// use std::convert::TryFrom;
/// use clf::pattern::RegexBundle;
///
/// let regs = RegexBundle::try_from(vec!["^#".to_string(), ";$".to_string()]).unwrap();
/// assert_eq!(regs.0.len(), 2);
/// ```
impl TryFrom<Vec<String>> for RegexBundle {
    type Error = AppError;

    fn try_from(list: Vec<String>) -> Result<Self, Self::Error> {
        let set = RegexSet::new(list)?;
        Ok(RegexBundle(set))
    }
}

/// A list of compiled regexes which will be used to match Unicode strings coming from
/// a logfile.
///
/// Each regex in this list will be tested for a potential match against an Unicode string,
/// coming from a log file. If any of this list matches, the list of regex captures
/// will be returned. But if a match is found also in the `exceptions` list, nothing
/// is returned.
#[derive(Debug, Deserialize)]
pub struct Pattern {
    /// a vector of compiled `Regex` structs which are hence all valid
    pub regexes: RegexVec,

    /// a `RegexSet` struct, as it's not necessary to get neither which regex triggers the match, nor
    /// capture groups
    pub exceptions: Option<RegexBundle>,
}

impl Pattern {
    /// Builds a `Pattern` set form a YAML string. Useful for unit tests, because this structure
    /// is used by the `Config` structure, directly loading the whole configuration from a YAML
    /// file.
    ///
    /// # Example
    ///
    /// ```rust
    /// use regex::Regex;
    /// use clf::pattern::Pattern;
    ///
    /// let mut yaml = r#"
    /// {
    ///     regexes: [
    ///         "^ERROR",
    ///         "FATAL",
    ///     "PANIC"
    ///     ],
    ///     exceptions: [
    ///         "^SLIGHT_ERROR",
    ///         "WARNING",
    ///     "NOT IMPORTANT$"
    ///     ]
    /// }"#;
    ///
    /// let p = Pattern::from_str(yaml).unwrap();
    /// assert_eq!(p.regexes.0.len(), 3);
    /// assert_eq!(p.exceptions.unwrap().0.len(), 3);
    /// ```
    pub fn from_str(yaml: &str) -> Result<Pattern, AppError> {
        let p: Pattern = serde_yaml::from_str(yaml)?;
        Ok(p)
    }

    /// Tests if `text` matches any of the regexes in the set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use regex::Regex;
    /// use clf::pattern::Pattern;
    ///
    /// let mut yaml = r#"
    /// {
    ///     regexes: [
    ///         "^ERROR",
    ///         "FATAL",
    ///         "PANIC",
    ///     ],
    ///     exceptions: [
    ///         "^SLIGHT_ERROR",
    ///         "WARNING",
    ///         "NOT IMPORTANT$",
    ///     ]
    /// }"#;
    ///
    /// let p = Pattern::from_str(yaml).unwrap();
    /// assert!(p.is_exception("this is NOT IMPORTANT"));
    /// ```
    pub fn is_exception(&self, text: &str) -> bool {
        self.exceptions
            .as_ref()
            .map_or(false, |x| x.0.is_match(text))
    }

    /// Try to find a match in the string `s` corresponding to the `regexes` list struct field,
    /// provided any regex in the exception list is not matched.
    ///
    /// # Example
    ///
    /// ```rust
    /// use regex::Regex;
    /// use clf::pattern::Pattern;
    ///
    /// let mut yaml = r#"
    /// {
    ///     regexes: [
    ///         '^\+?([0-9]{1,3})-([0-9]{3})-[0-9]{3}-[0-9]{4}$',
    ///         '^([0-9]{3})-[0-9]{3}-[0-9]{4}$',
    ///     ],
    /// }"#;
    ///
    /// let p = Pattern::from_str(yaml).unwrap();
    /// let mut caps = p.captures("541-754-3010").unwrap();
    /// assert_eq!(caps.get(1).unwrap().as_str(), "541");
    ///
    /// caps = p.captures("1-541-754-3010").unwrap();
    /// assert_eq!(caps.get(1).unwrap().as_str(), "1");   
    /// assert_eq!(caps.get(2).unwrap().as_str(), "541");   
    ///
    /// caps = p.captures("+1-541-754-3010").unwrap();
    /// assert_eq!(caps.get(1).unwrap().as_str(), "1");   
    /// assert_eq!(caps.get(2).unwrap().as_str(), "541");
    ///
    /// assert!(p.captures("foo").is_none());   
    /// ```
    pub fn captures<'t>(&self, text: &'t str) -> Option<Captures<'t>> {
        // use RegexSet first
        for re in &self.regexes.0 {
            let caps = re.captures(text);

            // test if this is an exception
            if self.is_exception(text) {
                return None;
            }

            // we ended up here, so return the captures
            if caps.is_some() {
                return caps;
            }
        }
        None
    }
}

#[derive(Debug, Deserialize, PartialEq, Hash, Eq)]
#[allow(non_camel_case_types)]
//#[serde(try_from = "&str")]
pub enum PatternType {
    critical,
    warning,
    ok,
}

impl TryFrom<&str> for PatternType {
    type Error = AppError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "critical" => Ok(PatternType::critical),
            "warning" => Ok(PatternType::warning),
            "ok" => Ok(PatternType::ok),
            _ => Err(AppError::App {
                err: AppCustomErrorKind::UnsupportedPatternType,
                msg: format!("{} pattern type is not supported", s),
            }),
        }
    }
}

/// A structure combining patterns into 3 categories: *critical*, *warning* and *ok*.
// #[derive(Debug, Deserialize)]
// pub struct PatternSet {
//     pats: std::collections::HashMap<PatternType, Option<Pattern>>,
// }

/// A structure combining patterns into 3 categories: *critical*, *warning* and *ok*.
#[derive(Debug, Deserialize)]
pub struct PatternSet {
    pub critical: Option<Pattern>,
    pub warning: Option<Pattern>,
    pub ok: Option<Pattern>,
}

impl PatternSet {
    pub fn captures<'t>(&self, text: &'t str) -> Option<Captures<'t>> {
        // try to match critical pattern first
        if let Some(critical) = &self.critical {
            return critical.captures(text);
        }

        // and then warning
        if let Some(warning) = &self.warning {
            return warning.captures(text);
        }

        None
    }
}
