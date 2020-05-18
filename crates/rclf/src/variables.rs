//! This is where ad-hoc variables are stored. These are meant to be used from with the
//! configuration file.
use std::collections::HashMap;
//use std::ops::Deref;

use regex::Regex;

/// Variable name prefix to be inserted for each variable.
const VAR_PREFIX: &'static str = "CLF_";

/// Utility macro to easily add a variable into our struct.
// macro_rules! addvar {
//     ($hmap:ident, $var:expr, $value:expr) => {
//         $hmap.insert(format!("{}{}", VAR_PREFIX, $var), $value)
//     };
// }

/// A wrapper on a hashmap storing variable names, and their values.
#[derive(Debug)]
pub struct Vars(HashMap<String, String>);

impl Vars {
    /// Juste allocates a dedicated hashmap big enough to store all our variables.
    /// 30 should be enough.
    pub fn new() -> Vars {
        const NB_VARS: usize = 30;
        let hmap = HashMap::with_capacity(NB_VARS);

        // add 'static' variables
        //addvar!(hmap, "IP", "127.0.0.1".to_string());
        //addvar!(hmap, "HOSTNAME", hostname::get().unwrap().into_string().unwrap());

        Vars(hmap)
    }

    /// Just adds a new variable and its value.
    pub fn add_var<S: Into<String>>(&mut self, var: &str, value: S) {
        debug_assert!(!self.0.contains_key(var));
        self.0
            .insert(format!("{}{}", VAR_PREFIX, var), value.into());
    }

    /// Add variables taken from the capture group names or id.
    pub fn add_capture_groups(&mut self, re: &Regex, text: &str) {
        // get the captures
        let caps = re.captures(text).unwrap();

        // now loop and get text corresponding to either name or position
        for (i, cg_name) in re.capture_names().enumerate() {
            match cg_name {
                None => self.add_var(&format!("CAPTURE{}", i), caps.get(i).unwrap().as_str()),
                Some(cap_name) => self.add_var(cap_name, caps.name(cap_name).unwrap().as_str()),
            };
        }
    }

    /// Get a reference on inner hashmap.
    pub fn inner(&self) -> &HashMap<String, String> {
        &self.0
    }

    /// Replaces variables in the argument list and returns a new list where each arg is replaced, if any, by a variable's value.
    pub fn substitue(&self, old_args: &[&str]) -> Vec<String> {
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
        new_s.to_string()
    }
}

/// `Deref`and `DerefMut` traits implementation.
impl std::ops::Deref for Vars {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Vars {
    fn deref_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::*;

    #[test]
    fn variables() {
        let re = Regex::new(r"^([a-z\s]+) (\w+) (\w+) (?P<LASTNAME>\w+)").unwrap();
        let text = "my name is john fitzgerald kennedy, president of the USA";

        let mut v = Vars::new();
        v.add_capture_groups(&re, text);

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
        let mut v = Vars::new();
        v.add_capture_groups(&re, text);

        let args = &[
            "Hi, CLF_CAPTURE1",
            "(CLF_CAPTURE2 CLF_CAPTURE3) CLF_LASTNAME. I'm the president of the USA.",
        ];
        let new_args = v.substitue(args);

        assert_eq!(
            new_args,
            vec![
                "Hi, my name is".to_string(),
                "(john fitzgerald) kennedy. I'm the president of the USA.".to_string()
            ]
        );
    }
}
