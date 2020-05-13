//! This is where ad-hoc variables are stored. These are meant to be used from with the
//! configuration file.
use std::collections::HashMap;

/// A wrapper on a hashmap storing variable names, and their values.
pub struct Vars(HashMap<&'static str,String>);

impl Vars {
    /// Juste allocate a dedicate hashmap big enough to store all our variables.
    /// 30 should be enough.
    pub fn new() -> Vars {
        const NB_VARS: usize = 30;
        let mut hmap = HashMap::with_capacity(NB_VARS);

        // add 'static' variables
        hmap.insert("$IP", "127.0.0.1".to_string());

        Vars(hmap)
    }

    /// Just adds a new variable and its value.
    pub fn add_var<S: Into<String>>(&mut self, var: &'static str, value: S) {
        debug_assert!(!self.0.contains_key(var));
        self.0.insert(var, value.into());
    }
}