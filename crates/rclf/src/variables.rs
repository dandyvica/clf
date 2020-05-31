//! This is where ad-hoc variables are stored. These are meant to be used from with the
//! configuration file.
use log::trace;
use std::cmp::Eq;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use regex::Regex;

/// Variable name prefix to be inserted for each variable.
const VAR_PREFIX: &str = "CLF_";

#[derive(Debug)]
pub struct Variables<K, V>(HashMap<K, V>)
where
    K: Eq + Hash;

impl<K, V> Variables<K, V>
where
    K: Eq + Hash,
{
    pub fn inner(&self) -> &HashMap<K, V> {
        &self.0
    }
}

// /// `Deref`and `DerefMut` traits implementation.
impl<K, V> Deref for Variables<K, V>
where
    K: Eq + Hash,
{
    type Target = HashMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V> DerefMut for Variables<K, V>
where
    K: Eq + Hash,
{
    fn deref_mut(&mut self) -> &mut HashMap<K, V> {
        &mut self.0
    }
}

/// A wrapper on a hashmap storing variable names, and their values.
pub type RuntimeVariables = Variables<String, String>;

impl RuntimeVariables {
    /// Juste allocates a dedicated hashmap big enough (beefore needing reallocation) to store all our variables.
    /// 30 should be enough.
    pub fn new() -> Self {
        const NB_VARS: usize = 30;
        let hmap = HashMap::with_capacity(NB_VARS);

        // add 'static' variables
        //addvar!(hmap, "IP", "127.0.0.1".to_string());
        //addvar!(hmap, "HOSTNAME", hostname::get().unwrap().into_string().unwrap());

        Self(hmap)
    }

    /// Just adds a new variable and its value. The `Into` bound make it usable with `String` or `&str`.
    pub fn insert<S: Into<String>>(&mut self, var: &str, value: S) {
        debug_assert!(!self.0.contains_key(var));
        self.0
            .insert(format!("{}{}", VAR_PREFIX, var), value.into());
    }

    /// Add variables taken from the capture group names or ids.
    pub fn insert_captures(&mut self, re: &Regex, text: &str) {
        // get the captures
        let caps = re.captures(text).unwrap();
        trace!("caps={:?}", caps);

        // now loop and get text corresponding to either name or position
        for (i, cg_name) in re.capture_names().enumerate() {
            match cg_name {
                None => {
                    if let Some(cg) = caps.get(i) {
                        self.insert(&format!("CAPTURE{}", i), cg.as_str());
                    }
                }
                Some(cap_name) => self.insert(cap_name, caps.name(cap_name).unwrap().as_str()),
            };
        }
    }

    /// Replaces variables in the argument list and returns a new list where each arg is replaced, if any, by a variable's value.
    pub fn substitute(&self, old_args: &[&str]) -> Vec<String> {
        old_args
            .iter()
            .map(|arg| self.replace(arg))
            .collect::<Vec<String>>()
    }

    /// Replaces any occurence of a variable in the given string.
    fn replace(&self, s: &str) -> String {
        let mut new_s = String::from(s);

        for (var, value) in &self.0 {
            new_s = new_s.as_str().replace(var.as_str(), value.as_str());
        }
        new_s
    }

    // Add user defined variables into the variables namespace.
    //pub fn add_uservars(&mut self, user_vars: &UserVars) {}
}

/// User variables can be defined as part of the global namespace.
pub struct UserVars(HashMap<String, String>);

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::*;

    #[test]
    fn variables() {
        let re = Regex::new(r"^([a-z\s]+) (\w+) (\w+) (?P<LASTNAME>\w+)").unwrap();
        let text = "my name is john fitzgerald kennedy, president of the USA";

        let mut v = RuntimeVariables::new();
        v.insert_captures(&re, text);

        assert_eq!(
            v.0.get("CLF_CAPTURE0").unwrap(),
            "my name is john fitzgerald kennedy"
        );
        assert_eq!(v.0.get("CLF_CAPTURE1").unwrap(), "my name is");
        assert_eq!(v.0.get("CLF_CAPTURE2").unwrap(), "john");
        assert_eq!(v.0.get("CLF_CAPTURE3").unwrap(), "fitzgerald");
        assert_eq!(v.0.get("CLF_LASTNAME").unwrap(), "kennedy");

        println!("{:#?}", v.0);
    }

    #[test]
    fn replace() {
        let re = Regex::new(r"^([a-z\s]+) (\w+) (\w+) (?P<LASTNAME>\w+)").unwrap();
        let text = "my name is john fitzgerald kennedy, president of the USA";
        let mut v = RuntimeVariables::new();
        v.insert_captures(&re, text);

        let args = &[
            "Hi, CLF_CAPTURE1",
            "(CLF_CAPTURE2 CLF_CAPTURE3) CLF_LASTNAME. I'm the president of the USA.",
        ];
        let new_args = v.substitute(args);

        assert_eq!(
            new_args,
            vec![
                "Hi, my name is".to_string(),
                "(john fitzgerald) kennedy. I'm the president of the USA.".to_string()
            ]
        );
    }
}
