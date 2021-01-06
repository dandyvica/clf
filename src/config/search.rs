//! Contains the configuration each search.
use serde::Deserialize;

use super::{logfiledef::LogFileDef, tag::Tag};

#[derive(Debug, Deserialize, Clone)]
/// Contains the logfile attributes from the `LogFileDef` structure and all defined tags to search for patterns.
pub struct Search {
    /// the logfile name to check
    pub logfile: LogFileDef,

    /// a unique identifier for this search
    pub tags: Vec<Tag>,
}

impl Search {
    /// Return the list of all tag names
    pub fn tag_names(&self) -> Vec<&str> {
        self.tags.iter().map(|x| x.name.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::logfiledef::LogFileFormat;
    use std::path::PathBuf;

    #[test]
    fn search() {
        let yaml = r#"
logfile: 
    path: /var/log/kern.log
    format: json
    exclude: '^error'
tags: 
  - name: error
    options: "runcallback"
    process: false
    callback: { 
        script: "tests/scripts/echovars.py",
        args: ['arg1', 'arg2', 'arg3']
    }
    patterns:
        warning: {
            regexes: [
                'error',
            ],
            exceptions: [
                'STARTTLS'
            ]
        }        
            "#;

        let s: Search = serde_yaml::from_str(yaml).expect("unable to read YAML");

        assert_eq!(s.logfile.path(), &PathBuf::from("/var/log/kern.log"));
        assert_eq!(s.logfile.format, LogFileFormat::json);
        assert_eq!(s.logfile.exclude.unwrap().as_str(), "^error");

        assert_eq!(s.tags.len(), 1);
        let tag = s.tags.get(0).unwrap();
        assert_eq!(tag.name, "error");
        assert!(tag.options.runcallback);
        assert!(!tag.options.keepoutput);
        assert!(!tag.process);
        let script = std::path::PathBuf::from("tests/scripts/echovars.py");
        assert!(
            matches!(&tag.callback.as_ref().unwrap().callback, crate::config::callback::CallbackType::Script(Some(x)) if x == &script)
        );
        assert_eq!(
            tag.callback.as_ref().unwrap().args.as_ref().unwrap(),
            &["arg1", "arg2", "arg3"]
        );
    }
}
