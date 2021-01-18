//! A list of structures dedicated to match text data from a logfile. It merely defines a list of
//! regexes structures, which are used to search for a pattern in a text.
use std::convert::{From, TryFrom};
use std::iter::Sum;
use std::ops::Add;

use log::{debug, trace};
use regex::{Regex, RegexSet};
use serde::{Deserialize, Serialize};

use crate::context;
use crate::fromstr;
use crate::misc::error::{AppCustomErrorKind, AppError};

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
            v.push(Regex::new(re).map_err(|e| context!(e, "error in regex {}", re))?);
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
        let set = RegexSet::new(&list).map_err(|e| context!(e, "error in regexset {:?}", list))?;
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
    /// Tests if `text` matches any of the regexes in the set.
    fn is_exception(&self, text: &str) -> bool {
        self.exceptions
            .as_ref()
            .map_or(false, |x| x.0.is_match(text))
    }

    /// Try to find a match in the string `s` corresponding to the `regexes` list struct field,
    /// provided any regex in the exception list is not matched.
    fn is_match(&self, text: &str) -> Option<&Regex> {
        // dismiss exceptions at first
        if self.is_exception(text) {
            debug!("pattern exception occured for text: {}", text);
            return None;
        }

        // returns the first Regex involved in a match, None otherwise
        self.regexes.0.iter().find(|re| re.is_match(text))
    }
}

// Auto-implement `FromStr`
fromstr!(Pattern);

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
            _ => Err(AppError::new_custom(
                AppCustomErrorKind::UnsupportedPatternType,
                &format!("{} pattern type is not supported", s),
            )),
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
    pub critical: Option<Pattern>,
    pub warning: Option<Pattern>,
    pub ok: Option<Pattern>,
}

/// When a line is matched, this is used to store which pattern and which regex in the list triggered the match.
pub struct PatternMatchResult<'a> {
    pub pattern_type: PatternType,
    pub regex: &'a Regex,
}

impl<'a> PatternMatchResult<'a> {
    fn new(pattern_type: PatternType, regex: &'a Regex) -> Self {
        PatternMatchResult {
            pattern_type,
            regex,
        }
    }
}

impl PatternSet {
    /// Returns whether a critical or warning regex is involved in the match, provided no exception is matched.
    pub fn is_match(&self, text: &str) -> Option<PatternMatchResult> {
        // try to match critical pattern first
        if let Some(critical) = &self.critical {
            trace!("critical pattern is tried");
            let ret = critical
                .is_match(text)
                .map(|re| PatternMatchResult::new(PatternType::critical, re));
            if ret.is_some() {
                trace!("critical pattern is matching");
                return ret;
            }
        }

        // and then warning
        if let Some(warning) = &self.warning {
            trace!("warning pattern is tried");
            let ret = warning
                .is_match(text)
                .map(|re| PatternMatchResult::new(PatternType::warning, re));
            if ret.is_some() {
                trace!("warning pattern is matching");
                return ret;
            }
        }

        // and finally ok
        if let Some(ok) = &self.ok {
            trace!("ok pattern is tried");
            let ret = ok
                .is_match(text)
                .map(|re| PatternMatchResult::new(PatternType::ok, re));
            if ret.is_some() {
                trace!("ok pattern is matching");
                return ret;
            }
        }

        None
    }
}

// Auto-implement FromStr
fromstr!(PatternSet);

/// Holds, during the search, all counters relative to the pattern type
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct PatternCounters {
    pub critical_count: u64,
    pub warning_count: u64,
    pub ok_count: u64,
    pub exec_count: u64,
}

/// Sum is used to sum all counters of run data
impl<'a> Sum<&'a Self> for PatternCounters {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a Self>,
    {
        iter.fold(Self::default(), |a, b| Self {
            critical_count: a.critical_count + b.critical_count,
            warning_count: a.warning_count + b.warning_count,
            ok_count: a.ok_count + b.ok_count,
            exec_count: a.exec_count + b.exec_count,
        })
    }
}

/// Used in calculation all all counters for Nagios global exit
impl Add for PatternCounters {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            critical_count: self.critical_count + other.critical_count,
            warning_count: self.warning_count + other.warning_count,
            ok_count: self.ok_count + other.ok_count,
            exec_count: self.exec_count + other.exec_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;
    use std::str::FromStr;

    use super::*;

    #[test]
    fn pattern() {
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
        assert_eq!(p.exceptions.as_ref().unwrap().0.len(), 3);

        assert!(p.is_exception("this is NOT IMPORTANT"));

        let re = p.is_match("ERROR: core dump");
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
                regexes: ["ERROR$", "FATAL", "ABEND"]
                exceptions: ["^MINOR_ERROR", "WARNING", "NOT IMPORTANT$"]
            ok: 
                regexes: ["^RESET_ERROR", "RESET_FATAL", "RESET_FATAL"]
            "#;

        let p: PatternSet = serde_yaml::from_str(yaml).unwrap();

        // critical
        let match_text = p.is_match("ERROR: core dump ").unwrap();
        assert_eq!(match_text.pattern_type, PatternType::critical);
        assert_eq!(match_text.regex.as_str(), "^ERROR");
        assert!(p.is_match("SLIGHT_ERROR: core dump ").is_none());

        // warning
        let match_text = p.is_match("this is an ERROR").unwrap();
        assert_eq!(match_text.pattern_type, PatternType::warning);
        assert_eq!(match_text.regex.as_str(), "ERROR$");
        assert!(p.is_match("MINOR_ERROR: not a core dump ").is_none());

        // ok
        let match_text = p.is_match("RESET_ERROR: error is reset").unwrap();
        assert_eq!(match_text.pattern_type, PatternType::ok);
        assert_eq!(match_text.regex.as_str(), "^RESET_ERROR");
    }

    #[test]
    fn sum_counters() {
        let p = PatternCounters {
            critical_count: 1,
            warning_count: 2,
            ok_count: 3,
            exec_count: 4,
        };

        let v = vec![p; 10];
        let sum: PatternCounters = v.iter().sum();
        assert_eq!(sum.critical_count, 10);
        assert_eq!(sum.warning_count, 20);
        assert_eq!(sum.ok_count, 30);
        assert_eq!(sum.exec_count, 40);
    }

    #[test]
    fn add_counters() {
        let p1 = PatternCounters {
            critical_count: 1,
            warning_count: 2,
            ok_count: 3,
            exec_count: 4,
        };
        let p2 = PatternCounters {
            critical_count: 1,
            warning_count: 2,
            ok_count: 3,
            exec_count: 4,
        };

        let sum = p1 + p2;
        assert_eq!(sum.critical_count, 2);
        assert_eq!(sum.warning_count, 4);
        assert_eq!(sum.ok_count, 6);
        assert_eq!(sum.exec_count, 8);
    }
}
