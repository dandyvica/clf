//! This is where ad-hoc variables are stored. These are meant to be used from with the
//! configuration file.
use log::trace;
use std::collections::HashMap;
use std::fmt::Debug;

use regex::Regex;

/// Variable name prefix to be inserted for each variable.
const VAR_PREFIX: &str = "CLF_";

/// All variables, either runtime or user. These are provided to the callback.
#[derive(Debug)]
pub struct Variables {
    pub runtime_vars: HashMap<String, String>,
    pub user_vars: Option<HashMap<String, String>>,
}

impl Variables {
    /// Creates a new structure for variables.
    pub fn new() -> Self {
        const NB_VARS: usize = 30;

        Variables {
            runtime_vars: HashMap::with_capacity(NB_VARS),
            user_vars: None,
        }
    }

    /// Just adds a new variable and its value. The `Into` bound make it usable with `String` or `&str`.
    pub fn insert<S: Into<String>>(&mut self, var: &str, value: S) {
        debug_assert!(!self.runtime_vars.contains_key(var));
        self.runtime_vars
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

        for (var, value) in &self.runtime_vars {
            new_s = new_s.as_str().replace(var.as_str(), value.as_str());
        }
        new_s
    }

    // Add user defined variables into the variables namespace. Prepend user variables with prefix.
    pub fn insert_uservars(&mut self, user_vars: Option<HashMap<String, String>>) {
        if let Some(uservars) = user_vars {
            // allocate hashmap
            self.user_vars = Some(HashMap::<String, String>::new());

            // copy variables by prepending with prefix
            for (var, value) in uservars.iter() {
                self.user_vars
                    .as_mut()
                    .unwrap()
                    .insert(format!("{}{}", VAR_PREFIX, var), value.into());
            }
        };
    }
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

        let mut v = Variables::new();
        v.insert_captures(&re, text);

        assert_eq!(
            v.runtime_vars.get("CLF_CAPTURE0").unwrap(),
            "my name is john fitzgerald kennedy"
        );
        assert_eq!(v.runtime_vars.get("CLF_CAPTURE1").unwrap(), "my name is");
        assert_eq!(v.runtime_vars.get("CLF_CAPTURE2").unwrap(), "john");
        assert_eq!(v.runtime_vars.get("CLF_CAPTURE3").unwrap(), "fitzgerald");
        assert_eq!(v.runtime_vars.get("CLF_LASTNAME").unwrap(), "kennedy");

        println!("{:#?}", v.runtime_vars);
    }

    #[test]
    fn replace() {
        let re = Regex::new(r"^([a-z\s]+) (\w+) (\w+) (?P<LASTNAME>\w+)").unwrap();
        let text = "my name is john fitzgerald kennedy, president of the USA";
        let mut v = Variables::new();
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
