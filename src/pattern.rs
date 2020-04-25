/// A list of compiled regexes used to find matches into log files.
use std::collections::HashMap;
use std::convert::TryFrom;
//use std::convert::From;

use regex::{Captures, Regex, RegexSet};
use serde::{Deserialize, Serialize};

use crate::error::*;

//#[doc(hidden)]

#[derive(Debug, Deserialize)]
#[serde(try_from = "Vec<String>")]
pub struct RegexVec(pub Vec<Regex>);

#[derive(Debug, Deserialize)]
#[serde(try_from = "Vec<String>")]
pub struct RegexBundle(pub RegexSet);

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

impl TryFrom<Vec<String>> for RegexBundle {
    type Error = AppError;

    fn try_from(list: Vec<String>) -> Result<Self, Self::Error> {
        let set = RegexSet::new(list)?;
        Ok(RegexBundle(set))
    }
}

/// Defines the type of a search.
///
/// As a logfile can be searched for patterns, some are considered critical because
/// they need to be tackled ASAP, and some are just warning. *Ok* are meant to disable
/// previously matched patterns.
#[derive(Debug, Deserialize, PartialEq)]
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
                err: AppCustomError::UnsupportedPatternType,
                msg: format!("{} pattern type is not supported", s),
            }),
        }
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
    /// the type of a pattern, related to its severity
    pub r#type: PatternType,

    /// a vector of compiled *Regex* structs which are hence all valid
    pub regexes: RegexVec,

    /// a *RegexSet* struct, as it's not necessary to get neither which regex triggers the match, nor
    /// capture groups
    pub exceptions: Option<RegexBundle>,
}

impl Pattern {
    /// Builds a `Pattern` set form a JSON string. Useful for unit tests, because this structure
    /// is used by the `Config` structure, directly loading the whole configuration from a JSON
    /// file.
    ///
    /// # Example
    ///
    /// ```rust
    /// use regex::Regex;
    /// use clf::pattern::{Pattern, PatternType};
    ///
    /// let mut yaml = r#"
    /// {
    ///     type: critical,
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
    /// assert_eq!(p.r#type, PatternType::critical);
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
    /// use clf::pattern::{Pattern, PatternType};
    ///
    /// let mut yaml = r#"
    /// {
    ///     type: critical,
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

    /// Try to find a match in the string *s* corresponding to the *regex* list struct field,
    /// provided any regex in the exception list is not matched. If `use_set` is `true`, then
    /// the match is tried against the `RegexSet` field.
    ///
    /// # Example
    ///
    /// ```rust
    /// use regex::Regex;
    /// use clf::pattern::{Pattern, PatternType};
    /// 
    /// let mut yaml = r#"
    /// {
    ///     type: critical,
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

#[cfg(test)]
mod tests {
    use crate::pattern::{Pattern, PatternType};

    //#[test]
    // fn test_from_str() {
    //     // regexes are OK
    //     let mut re = Pattern::new(
    //         PatternType::critical,
    //         &vec![
    //             r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,4}\b",
    //             r"https?://(www\.)?[A-Za-z0-9]+\.(com|org|edu|gov|us)/?.*",
    //             r"^[0-9]{3}-[0-9]{2}-[0-9]{4}$",
    //         ],
    //         None,
    //     );
    //     assert!(re.is_ok());

    //     // error in regexes
    //     re = Pattern::new(
    //         PatternType::critical,
    //         &vec![
    //             r"\b[A-a-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,4}\b",
    //             r"https?://(www\.?[A-Za-z0-9]+\.(com|org|edu|gov|us)/?.*",
    //             r"^[0-9]{3}-[0-9]2}-[0-9]{4}$",
    //         ],
    //         None,
    //     );
    //     assert!(re.is_err());

    //     // match re.unwrap_err() {
    //     //     AppError::Regex(err) => assert_eq!(format!("{}", err), "foo"),
    //     //     _ => ()
    //     // };
    // }
}
