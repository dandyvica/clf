use regex::{Captures, Regex};
/// A list of compiled regexes used to find matches into log files.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Defines the string which is prepended to each capture, if any.
pub const CAPTURE_ROOT: &'static str = "$CLF_CAPTURE_";

/// Defines the type of a search.
///
/// As a logfile can be searched for patterns, some are considered critical because
/// they need to be tackled ASAP, and some are just warning. *Ok* are meant to disable
/// previously matched patterns.
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum PatternType {
    critical,
    warning,
    ok,
}

use crate::error::*;

/// A list of compiled regexes which will be used to match Unicode strings.
///
/// Each regex in this list will be tested for a potential match against an Unicode string,
/// coming from a log file. If any of this list matches, the list of regex captures
/// will be returned. But if a match is found also in the *exceptions* list, nothing
/// is returned.
#[derive(Debug, Serialize, Deserialize)]
pub struct Pattern<T> {
    /// the type of a pattern, related to its severity
    pub r#type: PatternType,

    /// a vector of compiled *Regex* structs which are hence all valid
    pub regexes: Vec<T>,

    /// a vector of *Regex* structs, but considered as exceptions regarding the previous list
    pub exceptions: Option<Vec<T>>,
}

impl Pattern<Regex> {
    /// Creates an new *Pattern* structure from a list of regex expressions, and optionally a list
    /// of regexes expressions exceptions.
    ///
    /// # Example
    ///
    /// ```rust
    /// use regex::Regex;
    /// use clf::pattern::{Pattern, PatternType};
    ///
    /// let mut re = Pattern::from_str(
    ///     &vec![
    ///         r"^([0-9]{3})-([0-9]{3})-([0-9]{4})$",
    ///         r"^([0-9]{2})-([0-9]{2})-([0-9]{2})-([0-9]{2})$"
    ///     ],
    ///     None,
    ///     PatternType::critical);
    /// assert_eq!(re.unwrap().regexes.len(), 2)
    /// ```
    pub fn from_str(
        re_list: &[&str],
        re_excp: Option<&[&str]>,
        ptype: PatternType,
    ) -> Result<Self, AppError> {
        let _re_list = Pattern::import(re_list)?;
        let _re_excp = match re_excp {
            Some(re) => Some(Pattern::import(re)?),
            None => None,
        };

        Ok(Pattern {
            regexes: _re_list,
            exceptions: _re_excp,
            r#type: ptype,
        })
    }

    /// Try to find a match in the string *s* corresponding to the *regex* list struct field,
    /// provided any regex in the exception list is not matched.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use regex::Regex;
    /// use clf::pattern::{Pattern, PatternType};
    ///
    /// let mut re = Pattern::from_str(
    ///     &vec![r"^([0-9]{3})-([0-9]{3})-([0-9]{4})$"],
    ///     None,
    ///     PatternType::critical).unwrap();
    /// assert_eq!(re.find("541-754-3010").unwrap().get("$CLF_CAPTURE_1").unwrap(), "541");
    /// ```
    pub fn find(&self, s: &str) -> Option<HashMap<String, String>> {
        for re in &self.regexes {
            let caps = re.captures(s);
            if caps.is_some() {
                if self.exceptions.is_some()
                    && self
                        .exceptions
                        .as_ref()
                        .unwrap()
                        .iter()
                        .any(|re| re.is_match(s))
                {
                    return None;
                } else {
                    return Some(Pattern::from_captures(caps.unwrap(), CAPTURE_ROOT));
                }
            }
        }
        None
    }

    /// Creates a vector of compiled *Regex* from a vector of regexes strings. Returns an error if any of the
    /// string expressions is not valid.
    ///
    /// # Example
    ///
    /// ```rust
    /// use regex::Regex;
    /// use clf::pattern::Pattern;
    ///
    /// let mut re = Pattern::import(&vec![
    ///     r"^([0-9]{3})-([0-9]{3})-([0-9]{4})$",
    ///     r"^([0-9]{2})-([0-9]{2})-([0-9]{2})-([0-9]{2})$"
    ///     ]);
    /// assert_eq!(re.unwrap().len(), 2);
    ///
    /// re = Pattern::import(&vec![
    ///     r"^([0-9]{3}-([0-9]{3})-([0-9]{4})$",
    ///     r"^([0-9]{2})-([0-9]{2})-([0-9]{2})-([0-9]{2})$"
    ///     ]);
    /// assert!(re.is_err());
    /// ```
    pub fn import(re_list: &[&str]) -> Result<Vec<Regex>, AppError> {
        let mut v = Vec::new();
        for re_expr in re_list {
            let re = Regex::new(re_expr)?;
            v.push(re);
        }
        Ok(v)
    }

    /// Creates a vector of strings from a *Regex* *Capture*.
    ///
    /// # Example
    ///
    /// ```rust
    /// use regex::Regex;
    /// use clf::pattern::{CAPTURE_ROOT, Pattern};
    ///
    /// let us_tel = Regex::new(r"^([0-9]{3})-([0-9]{3})-([0-9]{4})$").unwrap();
    /// let caps = us_tel.captures("541-754-3010").unwrap();
    /// let v = Pattern::from_captures(caps, CAPTURE_ROOT);
    /// assert_eq!(v.get("$CLF_CAPTURE_1").unwrap(), "541");
    /// ```
    pub fn from_captures<'t>(cap: Captures<'t>, root: &str) -> HashMap<String, String> {
        let mut hmap = HashMap::new();
        for i in 1..cap.len() {
            if let Some(m) = cap.get(i) {
                hmap.insert(format!("{}{}", root, i), m.as_str().to_string());
            }
        }
        hmap
    }
}

#[cfg(test)]
mod tests {
    use crate::pattern::{Pattern, PatternType};

    #[test]
    fn test_from_str() {
        // regexes are OK
        let mut re = Pattern::from_str(
            &vec![
                r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,4}\b",
                r"https?://(www\.)?[A-Za-z0-9]+\.(com|org|edu|gov|us)/?.*",
                r"^[0-9]{3}-[0-9]{2}-[0-9]{4}$",
            ],
            None,
            PatternType::critical,
        );
        assert!(re.is_ok());

        // error in regexes
        re = Pattern::from_str(
            &vec![
                r"\b[A-a-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,4}\b",
                r"https?://(www\.?[A-Za-z0-9]+\.(com|org|edu|gov|us)/?.*",
                r"^[0-9]{3}-[0-9]2}-[0-9]{4}$",
            ],
            None,
            PatternType::critical,
        );
        assert!(re.is_err());

        // match re.unwrap_err() {
        //     AppError::Regex(err) => assert_eq!(format!("{}", err), "foo"),
        //     _ => ()
        // };
    }
}
