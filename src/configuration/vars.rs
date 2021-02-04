//! Contains the definition of all structures for handling variables, either user-defined as used in the global tag, or
//! the runtime ones populated each time a pattern is matched.
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::misc::util::DEFAULT_CONTAINER_CAPACITY;

use super::pattern::PatternType;

/// Macro to build a variable name prepended with its prefix
#[macro_export]
macro_rules! prefix_var {
    // prefix_var!("LOGFILE") => "CLF_LOGFILE" as &str
    ($v:literal) => {
        concat!("CLF_", $v)
    };

    // prefix_var!(name) => "CLF_LOGFILE" as a String
    ($v:expr) => {
        Cow::from(format!("CLF_{}", $v))
    };

    // prefix_var!("CAPTURE", 2) => "CLF_CAPTURE2" as a String
    ($n:literal, $i:ident) => {
        Cow::from(format!("CLF_{}{}", $n, $i))
    };
}

// A variable sent through a JSON string could be either a string or an integer
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum VarType<T> {
    Str(T),
    Int(u64),
}

impl<T> VarType<T> {
    #[cfg(test)]
    pub fn as_t(&self) -> &T {
        match self {
            VarType::Str(s) => s,
            VarType::Int(_) => unimplemented!("VarType is not an int here!"),
        }
    }
}
impl<'a> VarType<&'a str> {
    pub fn to_string(&self) -> String {
        match self {
            VarType::Str(s) => s.to_string(),
            VarType::Int(i) => i.to_string(),
        }
    }
}

// these are all convertion helpers
impl<T> From<u64> for VarType<T> {
    fn from(i: u64) -> Self {
        VarType::Int(i)
    }
}

impl<T> From<usize> for VarType<T> {
    fn from(i: usize) -> Self {
        VarType::Int(i as u64)
    }
}

impl<'a> From<&'a str> for VarType<&'a str> {
    fn from(s: &'a str) -> Self {
        VarType::Str(s)
    }
}

impl<'a> From<&'a str> for VarType<String> {
    fn from(s: &'a str) -> Self {
        VarType::Str(s.to_string())
    }
}

impl<'a> From<&PatternType> for VarType<&'a str> {
    fn from(s: &PatternType) -> Self {
        VarType::Str(<&str>::from(s))
    }
}

impl<'a> From<&'a Cow<'_, str>> for VarType<&'a str> {
    fn from(s: &'a Cow<'_, str>) -> Self {
        VarType::Str(s)
    }
}

// Display is used for displaying variables for bypass reader
impl<T: Display> Display for VarType<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VarType::Str(s) => write!(f, "{}", s),
            VarType::Int(i) => write!(f, "{}", i),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
/// A generic variable structure.
pub struct Vars<K: Hash + Eq, V> {
    #[serde(flatten)]
    inner: HashMap<K, V>,
}

/// Runtime vars are created for each matched line. Using `Cow` minimizes string allocations.
pub type RuntimeVars<'a> = Vars<Cow<'a, str>, VarType<&'a str>>;

/// user vars are optionally defined in the global configuration tag.
pub type GlobalVars = HashMap<String, String>;

impl<K: Hash + Eq, V> Default for Vars<K, V> {
    fn default() -> Self {
        Vars {
            inner: HashMap::with_capacity(DEFAULT_CONTAINER_CAPACITY),
        }
    }
}

// Display is using when using the BypassReader to print out all capture groups
impl<K: Hash + Eq + Display, V: Display> Display for Vars<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = String::with_capacity(80);

        for (k, v) in &self.inner {
            output += &format!("{}:{}", k, v);
        }

        // write final output to formatter
        write!(f, "{}", output)
    }
}

/// As user variables are mainly used, just defer the `runtime_vars` field.
impl<K: Hash + Eq, V> Deref for Vars<K, V> {
    type Target = HashMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<K: Hash + Eq, V> DerefMut for Vars<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<K, V> Vars<K, V>
where
    K: std::cmp::Eq,
    K: std::hash::Hash,
    K: std::fmt::Display,
{
    // /// generic insertion of a variable
    // pub fn insert_var<S: Into<K>>(&mut self, var_name: S, value: V) {
    //     self.insert(var_name.into(), value);
    // }

    pub fn inner(&self) -> &HashMap<K, V> {
        &self.inner
    }
}

/// This implementation is made for lowering the number of memory allocations due to adding
/// new strings at each line of a logfile.
impl<'a> Vars<Cow<'a, str>, VarType<&'a str>> {
    /// This function could be either used with strings or integers
    pub fn insert_runtime_var<T: Into<VarType<&'a str>>>(&mut self, name: &'a str, value: T) {
        self.inner.insert(Cow::from(name), value.into());
    }

    /// Add variables taken from the capture group names or ids.
    pub fn insert_captures(&mut self, re: &Regex, text: &'a str) -> usize {
        // get the captures
        let caps = re.captures(text).unwrap();

        // insert number of captures
        let nbcaps = caps.len();

        // now loop and get text corresponding to either name or position
        for (i, cg_name) in re.capture_names().enumerate() {
            match cg_name {
                None => {
                    if let Some(m) = caps.get(i) {
                        // variable will be: CLF_CAPTURE2 (example)
                        self.inner
                            .insert(prefix_var!("CG_", i), VarType::from(m.as_str()));
                    }
                }
                Some(cap_name) => {
                    if let Some(m) = caps.name(cap_name) {
                        // variable will be: CLF_FOO (example)
                        self.inner
                            .insert(prefix_var!("CG_", cap_name), VarType::from(m.as_str()));
                    }
                }
            }
        }

        nbcaps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix() {
        assert_eq!(prefix_var!("LOGFILE"), Cow::from("CLF_LOGFILE"));

        let tag = String::from("TAG");
        assert_eq!(prefix_var!(tag), Cow::from("CLF_TAG"));

        let i = 2;
        assert_eq!(prefix_var!("CAPTURE", i), Cow::from("CLF_CAPTURE2"));
    }

    #[test]
    fn vars() {
        let mut vars = Vars::<Cow<str>, &str>::default();

        let a = Cow::from(prefix_var!("VAR1"));

        let var_name = String::from("VAR2");
        let b = Cow::from(prefix_var!(var_name));

        vars.insert(a, "this is a");
        vars.insert(b, "this is b");

        // create a dedicated JSON structure
        let json = serde_json::json!({ "vars": vars }).to_string();

        println!("{:#?}", json);
    }

    #[test]
    fn variables() {
        // create dummy variables
        let re = Regex::new(r"^([a-z\s]+) (\w+) (\w+) (?P<LASTNAME>\w+)").unwrap();
        let text = "my name is john fitzgerald kennedy, president of the USA";

        let mut vars = RuntimeVars::default();
        vars.insert_captures(&re, text);

        assert!(
            matches!(vars.get("CLF_CG_0").unwrap(), VarType::Str(x) if x == &"my name is john fitzgerald kennedy")
        );
        assert!(matches!(vars.get("CLF_CG_1").unwrap(), VarType::Str(x) if x == &"my name is"));
        assert!(matches!(vars.get("CLF_CG_2").unwrap(), VarType::Str(x) if x == &"john"));
        assert!(matches!(vars.get("CLF_CG_3").unwrap(), VarType::Str(x) if x == &"fitzgerald"));
        assert!(matches!(vars.get("CLF_CG_LASTNAME").unwrap(), VarType::Str(x) if x == &"kennedy"));

        vars.insert_runtime_var("CLF_LOGFILE", "/var/log/foo");
        vars.insert_runtime_var("CLF_TAG", "tag1");

        assert!(vars.contains_key("CLF_LOGFILE"));
        assert!(vars.contains_key("CLF_TAG"));

        // check json
        let _json = serde_json::json!({ "vars": vars }).to_string();
        //println!("{:#?}", json);
    }
}
