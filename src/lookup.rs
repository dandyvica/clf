use regex::{Captures, Regex};
use serde::{Deserialize, Serialize};

pub trait Lookup<T> {
    fn lookup(&self, line: &str) -> Option<T>;
}

impl Lookup<bool> for Regex {
    fn lookup(&self, line: &str) -> Option<bool> {
        match self.is_match(line) {
            false => None,
            true => Some(true),
        }
    }
}

impl<'t> Lookup<Vec<String>> for Regex {
    fn lookup(&self, line: &str) -> Option<Vec<String>> {
        match self.captures(line) {
            None => None,
            Some(caps) => {
                let v: Vec<_> = caps
                    .iter()
                    .map(|x| x.unwrap().as_str().to_string())
                    .collect();
                Some(v)
            }
        }
    }
}
