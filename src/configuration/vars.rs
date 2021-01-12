//! Contains the definition of all structures for handling variables, either user-defined as used in the global tag, or
//! the runtime ones populated each time a pattern is matched.
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::misc::constants::DEFAULT_CONTAINER_CAPACITY;

/// Macro to build a variable name prepended with its prefix
#[macro_export]
macro_rules! prefix_var {
    // prefix_var!("LOGFILE") => "CLF_LOGFILE" as &str
    ($v:literal) => {
        Cow::from(concat!("CLF_", $v))
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

#[derive(Debug, Serialize, Deserialize, Clone)]
/// A generic variable structure.
pub struct Vars<K: Hash + Eq, V>(HashMap<K, V>);

/// Runtime vars are created for each matched line. Using `Cow` minimizes string allocations.
pub type RuntimeVars<'a> = Vars<Cow<'a, str>, &'a str>;

/// user vars are optionally defined in the global configuration tag.
pub type GlobalVars = Vars<String, String>;

impl<K: Hash + Eq, V> Default for Vars<K, V> {
    fn default() -> Self {
        Vars(HashMap::with_capacity(DEFAULT_CONTAINER_CAPACITY))
    }
}

// Display is using when using the BypassReader to print out all capture groups
impl<K: Hash + Eq + Display, V: Display> Display for Vars<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = String::with_capacity(80);

        for (k, v) in &self.0 {
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
        &self.0
    }
}

impl<K: Hash + Eq, V> DerefMut for Vars<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K, V> Vars<K, V>
where
    K: std::cmp::Eq,
    K: std::hash::Hash,
    K: std::fmt::Display,
{
    /// generic insertion of a variable
    pub fn insert_var<S: Into<K>>(&mut self, var_name: S, value: V) {
        self.insert(var_name.into(), value);
    }

    pub fn inner(&self) -> &HashMap<K, V> {
        &self.0
    }
}

/// This implementation is made for lowering the number of memory allocations due to adding
/// new strings at each line of a logfile.
impl<'a> Vars<Cow<'a, str>, &'a str> {
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
                        self.0.insert(prefix_var!("CG_", i), m.as_str());
                    }
                }
                Some(cap_name) => {
                    if let Some(m) = caps.name(cap_name) {
                        // variable will be: CLF_FOO (example)
                        self.0.insert(prefix_var!("CG_", cap_name), m.as_str());
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

        let mut vars = Vars::<Cow<str>, &str>::default();
        vars.insert_captures(&re, text);

        assert_eq!(
            vars.get("CLF_CG_0").unwrap(),
            &"my name is john fitzgerald kennedy"
        );
        assert_eq!(vars.get("CLF_CG_1").unwrap(), &"my name is");
        assert_eq!(vars.get("CLF_CG_2").unwrap(), &"john");
        assert_eq!(vars.get("CLF_CG_3").unwrap(), &"fitzgerald");
        assert_eq!(vars.get("CLF_CG_LASTNAME").unwrap(), &"kennedy");

        vars.insert_var("CLF_LOGFILE", "/var/log/foo");
        vars.insert_var(String::from("CLF_TAG"), "tag1");

        assert!(vars.contains_key("CLF_LOGFILE"));
        assert!(vars.contains_key("CLF_TAG"));

        // check json
        let _json = serde_json::json!({ "vars": vars }).to_string();
        //println!("{:#?}", json);
    }
}
