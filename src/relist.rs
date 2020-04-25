use std::convert::TryFrom;

use regex::{Captures, Regex, RegexSet};
use serde::{Deserialize, Serialize};

use crate::error::*;

#[derive(Debug, Deserialize)]
#[serde(try_from = "RegexList<Vec<String>>")]
pub struct RegexVec(pub Vec<Regex>);

#[derive(Debug, Deserialize)]
#[serde(try_from = "RegexList<Vec<String>>")]
pub struct RegexBundle(pub RegexSet);

#[derive(Debug, Deserialize)]
pub struct RegexList<T>(T);

///
/// # Example
///
/// ```rust
/// use std::convert::TryFrom;
/// use regex::Regex;
/// use clf::pattern::{RegexList, RegexVec};
///
/// let re = RegexList::try_from(vec![
///     r"^([0-9]{3})-([0-9]{3})-([0-9]{4})$".to_string(),
///     r"^([0-9]{2})-([0-9]{2})-([0-9]{2})-([0-9]{2})$".to_string()
///     ]).unwrap();
/// assert_eq!(re.0.len(), 2);
///
/// ```
impl TryFrom<RegexList<Vec<String>>> for RegexVec {
    type Error = AppError;

    fn try_from(list: RegexList<Vec<String>>) -> Result<Self, Self::Error> {
        let mut v: Vec<Regex> = Vec::new();
        for re in &list.0 {
            v.push(Regex::new(re)?);
        }
        Ok(RegexVec(v))
    }
}

impl TryFrom<RegexList<Vec<String>>> for RegexBundle {
    type Error = AppError;

    fn try_from(list: RegexList<Vec<String>>) -> Result<Self, Self::Error> {
        let set = RegexSet::new(list.0)?;
        Ok(RegexBundle(set))
    }
}

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
    // // pub fn new(
    // //     ptype: PatternType,
    // //     re_list: &[String],
    // //     re_excp: Option<&[String]>,
    // // ) -> Result<Self, AppError> {
    // //     // convert to a list of regexes
    // //     let compiled_list = RegexList::try_from(RegexVec(re_list.clone()))?;
// 
    // //     // the set contains all compiled regexes
    // //     let compiled_set = RegexList::try_from(RegexBundle(re_list))?;
// 
    // //     // beware that exception list could be None
    // //     let exception_list = match re_excp {
    // //         Some(x) => Some(RegexSet::new(x)?),
    // //         None => None,
    // //     };
// 
    // //     Ok(Pattern {
    // //         regexes: compiled_list,
    // //         //regexset: compiled_set,
    // //         exceptions: exception_list,
    // //         r#type: ptype,
    // //     })
    // // }
