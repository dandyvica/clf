//! This is where ad-hoc variables are stored. These are meant to be used from with the
//! configuration file.
use log::trace;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::misc::util::Cons;

/// Macro to build a variable name prepended with its prefix
#[macro_export]
macro_rules! var {
    ($v:literal) => {
        format!("{}{}", Cons::VAR_PREFIX, $v)
    };
}

/// All variables, either runtime or user. These are provided to the callback.
#[derive(Debug, Serialize, Deserialize)]
pub struct Variables {
    // variables created during logfile analysis, like capture groups or line being read
    runtime_vars: HashMap<String, String>,

    // user-defined variables in the config file
    // this serde attribute keeps the serializing ok when Option is None
    #[serde(skip_serializing_if = "Option::is_none")]
    user_vars: Option<HashMap<String, String>>,
}

/// As user variables are mainly used, just defer the `runtime_vars` field.
impl Deref for Variables {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.runtime_vars
    }
}

impl DerefMut for Variables {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.runtime_vars
    }
}

impl Default for Variables {
    fn default() -> Self {
        Variables {
            runtime_vars: HashMap::with_capacity(Cons::DEFAULT_CONTAINER_CAPACITY),
            user_vars: None,
        }
    }
}

impl Variables {
    /// Only keeps variables matching the values passed in `vars` slice.
    #[inline(always)]
    pub fn retain(&mut self, vars: &[&str]) {
        self.runtime_vars.retain(|k, _| vars.contains(&k.as_str()));
        debug_assert!(self.runtime_vars.len() == vars.len());
    }

    /// Returns all runtime_vars
    #[inline(always)]
    pub fn runtime_vars(&self) -> &HashMap<String, String> {
        &self.runtime_vars
    }

    /// Returns user_vars
    #[inline(always)]
    pub fn user_vars(&self) -> &Option<HashMap<String, String>> {
        &self.user_vars
    }

    /// Just adds a new variable and its value. The `Into` bound make it usable with `String` or `&str`.
    pub fn insert<S: Into<String>>(&mut self, var: &str, value: S) {
        debug_assert!(!self.runtime_vars.contains_key(var));
        self.runtime_vars
            .insert(format!("{}{}", Cons::VAR_PREFIX, var), value.into());
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
    #[cfg(test)]
    pub fn substitute(&self, old_args: &[&str]) -> Vec<String> {
        old_args
            .iter()
            .map(|arg| self.replace(arg))
            .collect::<Vec<String>>()
    }

    /// Replaces any occurence of a variable in the given string.
    #[cfg(test)]
    fn replace(&self, s: &str) -> String {
        let mut new_s = String::from(s);

        for (var, value) in &self.runtime_vars {
            new_s = new_s.as_str().replace(var.as_str(), value.as_str());
        }
        new_s
    }

    /// Adds user defined variables into the variables namespace. Prepend user variables with prefix.
    pub fn insert_uservars(&mut self, user_vars: Option<HashMap<String, String>>) {
        if let Some(uservars) = user_vars {
            // allocate hashmap
            self.user_vars = Some(HashMap::<String, String>::new());

            // copy variables by prepending with prefix
            for (var, value) in uservars.iter() {
                self.user_vars
                    .as_mut()
                    .unwrap()
                    .insert(format!("{}{}", Cons::VAR_PREFIX, var), value.into());
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::data::sample_vars;

    //#[test]
    fn variables() {
        let mut v = sample_vars();

        assert_eq!(
            v.runtime_vars().get("CLF_CAPTURE0").unwrap(),
            "my name is john fitzgerald kennedy"
        );
        assert_eq!(v.get("CLF_CAPTURE1").unwrap(), "my name is");
        assert_eq!(v.get("CLF_CAPTURE2").unwrap(), "john");
        assert_eq!(v.get("CLF_CAPTURE3").unwrap(), "fitzgerald");
        assert_eq!(v.get("CLF_LASTNAME").unwrap(), "kennedy");

        v.insert("LOGFILE", "/var/log/foo");
        v.insert("TAG", "tag1");

        v.retain(&[&var!("LOGFILE"), &var!("TAG")]);
        //println!("{:#?}", v.runtime_vars);
        assert!(v.contains_key("CLF_LOGFILE"));
        assert!(v.contains_key("CLF_TAG"));
        assert!(!v.contains_key("CLF_CAPTURE2"));
    }

    #[test]
    fn replace() {
        let v = sample_vars();

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
