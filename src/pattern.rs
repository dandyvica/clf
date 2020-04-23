/// A list of compiled regexes used to find matches into log files.
use std::collections::HashMap;

use regex::{Captures, Regex, RegexSet};
use serde::{Deserialize, Serialize};

/// Defines the string which is prepended to each capture, if any.
pub const CAPTURE_ROOT: &'static str = "$CLF_CAPTURE_";

/// A trick to mimic TryFrom trait for Vec<String>. Otherwise, rustc error E0117 is raised.
/// Works with &[String] or &[&str]
///
/// # Example
///
/// ```rust
/// use regex::Regex;
/// use clf::pattern::try_from;
///
/// let mut re = try_from(&vec![
///     r"^([0-9]{3})-([0-9]{3})-([0-9]{4})$",
///     r"^([0-9]{2})-([0-9]{2})-([0-9]{2})-([0-9]{2})$"
///     ]);
/// assert_eq!(re.unwrap().len(), 2);
///
/// re = try_from(&vec![
///     r"^([0-9]{3}-([0-9]{3})-([0-9]{4})$",
///     r"^([0-9]{2})-([0-9]{2})-([0-9]{2})-([0-9]{2})$"
///     ]);
/// assert!(re.is_err());
/// ```
#[doc(hidden)]
pub fn try_from<T: AsRef<str>>(list: &[T]) -> Result<Vec<Regex>, AppError> {
    let mut v: Vec<Regex> = Vec::new();
    for re in list {
        let compiled_re = Regex::new(re.as_ref())?;
        v.push(compiled_re);
    }
    Ok(v)
}

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
#[derive(Debug, Deserialize)]
pub struct Pattern<T, U> {
    /// the type of a pattern, related to its severity
    pub r#type: PatternType,

    /// a vector of compiled *Regex* structs which are hence all valid
    pub regexes: Vec<T>,

    /// A RegexSet which contains all compiled regexes from the regexes Vec.
    #[serde(skip)]
    pub regexset: U,

    /// a *RegexSet* struct, as it's not necessary to get neither which regex triggers the match, nor
    /// capture groups
    pub exceptions: Option<U>,
}

impl Pattern<Regex, RegexSet> {
    /// Creates an new *Pattern* structure from a list of regex expressions, and optionally a list
    /// of regexes expressions exceptions.
    ///
    /// # Example
    ///
    /// ```rust
    /// use regex::Regex;
    /// use clf::pattern::{Pattern, PatternType};
    ///
    /// let mut re = Pattern::new(
    ///     PatternType::critical,
    ///     &vec![
    ///         r"^([0-9]{3})-([0-9]{3})-([0-9]{4})$",
    ///         r"^([0-9]{2})-([0-9]{2})-([0-9]{2})-([0-9]{2})$"
    ///     ],
    ///     None);
    /// assert_eq!(re.unwrap().regexes.len(), 2)
    /// ```
    pub fn new<T: AsRef<str>>(
        ptype: PatternType,
        re_list: &[T],
        re_excp: Option<&[T]>,
    ) -> Result<Self, AppError> {
        // convert to a list of regexes
        let compiled_list = try_from(re_list)?;

        // the set contains all compiled regexes
        let compiled_set = RegexSet::new(re_list)?;

        // beware that exception list could be None
        let exception_list = match re_excp {
            Some(x) => Some(RegexSet::new(x)?),
            None => None,
        };

        Ok(Pattern {
            regexes: compiled_list,
            regexset: compiled_set,
            exceptions: exception_list,
            r#type: ptype,
        })
    }

    /// Tests if `text` matches any of the regexes in the set
    fn is_exception(&self, text: &str) -> bool {
        self.exceptions.as_ref().map_or(false, |x| x.is_match(text))
    }

    /// Try to find a match in the string *s* corresponding to the *regex* list struct field,
    /// provided any regex in the exception list is not matched. If `use_set` is `true`, then
    /// the match is tried against the `RegexSet` field.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use regex::Regex;
    /// use clf::pattern::{Pattern, PatternType};
    ///
    /// let mut re = Pattern::new(
    ///     PatternType::critical,
    ///     &vec![r"^([0-9]{3})-([0-9]{3})-([0-9]{4})$"],
    ///     None).unwrap();
    /// assert!(re.captures("541-754-3010", false).is_some());
    /// ```
    pub fn captures<'t>(&self, text: &'t str, use_set: bool) -> Option<Captures<'t>> {
        // use RegexSet first
        if use_set {
            let matches = self.regexset.matches(text);

            // no match
            if !matches.matched_any() {
                return None;
            }

            // test if this is an exception
            if self.is_exception(text) {
                return None;
            }

            // if we ended up here, text matched and no exception is set
            let matched_id = match matches.iter().nth(0) {
                None => panic!("RegexSet id is empty but shouldn't!, {}", line!()),
                Some(i) => i,
            };

            // try to get the equivalent already compiled regex
            let matched_regex = match self.regexes.get(matched_id) {
                None => panic!("RegexSet id is not found!, {}", line!()),
                Some(i) => i,
            };

            // now we can safely call
            matched_regex.captures(text)
        }
        // or use the simple Vec
        else {
            for re in &self.regexes {
                let caps = re.captures(text);
                if caps.is_none() {
                    return None;
                }

                // test if this is an exception
                if self.is_exception(text) {
                    return None;
                }

                // we ended up here, so return the captures
                return caps;
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::pattern::{Pattern, PatternType};

    #[test]
    fn test_from_str() {
        // regexes are OK
        let mut re = Pattern::new(
            PatternType::critical,
            &vec![
                r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,4}\b",
                r"https?://(www\.)?[A-Za-z0-9]+\.(com|org|edu|gov|us)/?.*",
                r"^[0-9]{3}-[0-9]{2}-[0-9]{4}$",
            ],
            None,
        );
        assert!(re.is_ok());

        // error in regexes
        re = Pattern::new(
            PatternType::critical,
            &vec![
                r"\b[A-a-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,4}\b",
                r"https?://(www\.?[A-Za-z0-9]+\.(com|org|edu|gov|us)/?.*",
                r"^[0-9]{3}-[0-9]2}-[0-9]{4}$",
            ],
            None,
        );
        assert!(re.is_err());

        // match re.unwrap_err() {
        //     AppError::Regex(err) => assert_eq!(format!("{}", err), "foo"),
        //     _ => ()
        // };
    }
}
