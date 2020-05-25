//! A list of structures dedicated to match text data from a logfile. It merely defines a list of
//! regexes structures, which are used to search for a pattern in a text.
//!
use std::convert::{From, TryFrom};

use regex::{Regex, RegexSet};
use serde::Deserialize;
//use pcre2::Regex;

use crate::error::{AppCustomErrorKind, AppError};

/// A helper structure for deserializing into a `RegexVec` automatically from a `Vec<String>`.
#[derive(Debug, Deserialize, Clone)]
#[serde(try_from = "Vec<String>")]
pub struct RegexVec(Vec<Regex>);

/// A helper structure for deserializing into a `RegexSet` automatically from a `Vec<String>`.
#[derive(Debug, Deserialize, Clone)]
#[serde(try_from = "Vec<String>")]
pub struct RegexBundle(RegexSet);

/// An implementation of `TryFrom` for the helper tuple struct `RegexVec`.
///
/// This just creates a `RegexVec` structure from a vector of regexes strings. This is
/// used by the `serde` deserialize process in order to automatically transforms a vector
/// of strings read from the YAML config file into a `RegexVec` structure, which contains
/// a vector of compiled `Regex` structs.
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
#[derive(Debug, Deserialize, Clone)]
pub struct Pattern {
    /// A vector of compiled `Regex` structs which are hence all valid.
    regexes: RegexVec,

    /// A `RegexSet` struct, as it's not necessary to get neither which regex triggers the match, nor
    /// capture groups.
    exceptions: Option<RegexBundle>,
}

impl Pattern {
    /// Builds a `Pattern` set form a YAML string. Useful for unit tests, because this structure
    /// is used by the `Config` structure, directly loading the whole configuration from a YAML
    /// file.
    pub fn from_str(yaml: &str) -> Result<Pattern, AppError> {
        let p: Pattern = serde_yaml::from_str(yaml)?;
        Ok(p)
    }

    /// Tests if `text` matches any of the regexes in the set.
    pub fn is_exception(&self, text: &str) -> bool {
        self.exceptions
            .as_ref()
            .map_or(false, |x| x.0.is_match(text))
    }

    /// Try to find a match in the string `s` corresponding to the `regexes` list struct field,
    /// provided any regex in the exception list is not matched.
    pub fn is_match(&self, text: &str) -> Option<&Regex> {
        // dismiss exceptions at first
        if self.is_exception(text) {
            return None;
        }

        // returns the first Regex involved in a match, None otherwise
        self.regexes
            .0
            .iter()
            .find(|re| re.is_match(text))
            .and_then(|re| Some(re))
    }
}

#[derive(Debug, Deserialize, PartialEq, Hash, Eq)]
#[allow(non_camel_case_types)]
/// Qualification of `Pattern`.
pub enum PatternType {
    critical,
    warning,
    ok,
}

/// Simple implementation of `TryFrom`.
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

/// Converts a `PatternType` to a `String`.
impl From<PatternType> for String {
    fn from(pattern_type: PatternType) -> Self {
        let s = match pattern_type {
            PatternType::critical => "critical",
            PatternType::warning => "warning",
            PatternType::ok => "ok",
        };
        s.to_string()
    }
}

/// A structure combining patterns into 3 categories: *critical*, *warning* and *ok*.
#[derive(Debug, Deserialize, Clone)]
pub struct PatternSet {
    critical: Option<Pattern>,
    warning: Option<Pattern>,
    ok: Option<Pattern>,
}

impl PatternSet {
    /// Returns whether a critical or warning regex is involved in the match, provided no exception is matched.
    pub fn is_match(&self, text: &str) -> Option<(PatternType, &Regex)> {
        // try to match critical pattern first
        if let Some(critical) = &self.critical {
            return critical
                .is_match(text)
                .and_then(|re| Some((PatternType::critical, re)));
        }

        // and then warning
        if let Some(warning) = &self.warning {
            return warning
                .is_match(text)
                .and_then(|re| Some((PatternType::warning, re)));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use super::*;

    #[test]
    fn from_str() {
        let yaml = r#"
    {
        regexes: [
            "^ERROR",
            "FATAL",
            "PANIC"
        ],
        exceptions: [
            "^SLIGHT_ERROR",
            "WARNING",
            "NOT IMPORTANT$"
        ]
    }"#;
        let p = Pattern::from_str(yaml).unwrap();
        assert_eq!(p.regexes.0.len(), 3);
        assert_eq!(p.exceptions.unwrap().0.len(), 3);
    }

    #[test]
    fn is_exception() {
        let yaml = r#"
        {
            regexes: [
                "^ERROR",
                "FATAL",
                "PANIC",
            ],
            exceptions: [
                "^SLIGHT_ERROR",
                "WARNING",
                "NOT IMPORTANT$",
            ]
        }"#;

        let p = Pattern::from_str(yaml).unwrap();
        assert!(p.is_exception("this is NOT IMPORTANT"));
    }

    #[test]
    fn is_match() {
        let yaml = r#"
     {
         regexes: [
             '^\+?([0-9]{1,3})-([0-9]{3})-[0-9]{3}-[0-9]{4}$',
             '^([0-9]{3})-[0-9]{3}-[0-9]{4}$',
         ],
     }"#;

        let p = Pattern::from_str(yaml).unwrap();
        let re = p.is_match("541-754-3010");
        assert!(re.is_some());
    }

    #[test]
    fn try_from_patterntype() {
        let pt = PatternType::try_from("critical").unwrap();
        assert_eq!(pt, PatternType::critical);

        let pt_err = PatternType::try_from("foo");
        assert!(pt_err.is_err());
    }

    #[test]
    fn try_from_regexvec() {
        let regs = RegexVec::try_from(vec!["^#".to_string(), ";$".to_string()]).unwrap();
        assert_eq!(regs.0.len(), 2);

        let regs_err = RegexVec::try_from(vec!["(error".to_string()]);
        assert!(regs_err.is_err());
    }

    #[test]
    fn try_from_regexset() {
        let regs = RegexBundle::try_from(vec!["^#".to_string(), ";$".to_string()]).unwrap();
        assert_eq!(regs.0.len(), 2);

        let regs_err = RegexBundle::try_from(vec!["(error".to_string()]);
        assert!(regs_err.is_err());
    }
    #[test]
    fn pattern_set() {
        let yaml = r#"
    critical:
        regexes: ["^ERROR", "GnuPG", "PANIC", "WARNING"]
        exceptions: ["^SLIGHT_ERROR", "WARNING", "NOT IMPORTANT$"]
    warning:
        regexes: ["^ERROR", "FATAL", "PANIC"]
        exceptions: ["^SLIGHT_ERROR", "WARNING", "NOT IMPORTANT$"]
    ok: 
        regexes: ["^ERROR", "FATAL", "PANIC"]
    "#;

        let p: PatternSet = serde_yaml::from_str(yaml).unwrap();

        let match_text = p.is_match("ERROR: core dump ").unwrap();
        assert_eq!(match_text.0, PatternType::critical);

        assert!(p.is_match("WARNING: error dump ").is_none());
    }
}

// let mut yaml = r#"
// {
//     regexes: [
//         '^\+?([0-9]{1,3})-([0-9]{3})-[0-9]{3}-[0-9]{4}$',
//         '^([0-9]{3})-[0-9]{3}-[0-9]{4}$',
//     ],
// }"#;
//
// let p = Pattern::from_str(yaml).unwrap();
// let mut caps = p.captures("541-754-3010").unwrap();
// assert_eq!(caps.get(1).unwrap().as_str(), "541");
//
// caps = p.captures("1-541-754-3010").unwrap();
// assert_eq!(caps.get(1).unwrap().as_str(), "1");
// assert_eq!(caps.get(2).unwrap().as_str(), "541");
//
// caps = p.captures("+1-541-754-3010").unwrap();
// assert_eq!(caps.get(1).unwrap().as_str(), "1");
// assert_eq!(caps.get(2).unwrap().as_str(), "541");
//
// assert!(p.captures("foo").is_none());
