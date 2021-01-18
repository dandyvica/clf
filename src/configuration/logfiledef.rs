//! Contains the logfile configuration for each logfile. These are not related to a search but only to the logfile itself: format (plain or JSON), optional lines to exclude, etc.
use std::path::PathBuf;

use regex::Regex;
use serde::{de, Deserialize, Deserializer};
use serde_yaml::Value;

use super::archive::LogArchive;
use super::logsource::LogSource;
use crate::misc::nagios::NagiosError;

// a logfile could be of different format. Necessary to effectively read them
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[allow(non_camel_case_types)]
/// The format of a logfile, either plain or JSON.
pub enum LogFileFormat {
    plain,
    json,
}

impl Default for LogFileFormat {
    fn default() -> Self {
        LogFileFormat::plain
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
/// Logfile attributes not dependant from a runtime search.
pub struct LogFileDef {
    // full path of the logfile
    #[serde(flatten)]
    pub path: LogSource,

    // file format
    #[serde(default)]
    pub format: LogFileFormat,

    // exclude some lines
    #[serde(default)]
    #[serde(deserialize_with = "to_regex")]
    pub exclude: Option<Regex>,

    // optional archive file name. If not specified, itr's just the same file + .1
    pub archive: Option<LogArchive>,

    // what to expect when logfile is not accessible
    #[serde(default)]
    pub logfilemissing: NagiosError,
}

impl LogFileDef {
    /// Return the path variant from LogSource
    pub fn path(&self) -> &PathBuf {
        match &self.path {
            LogSource::LogFile(path) => path,
            LogSource::LogList(_) => unimplemented!(
                "LogSource::LogList not permitted here in {} !",
                module_path!()
            ),
        }
    }

    /// Get archive path
    pub fn archive_path(&self) -> PathBuf {
        match &self.archive {
            None => LogArchive::default_path(self.path()),
            Some(archive) => archive.archived_path(self.path()),
        }
    }

    // Return the list variant from LogSource
    #[cfg(test)]
    pub fn list(&self) -> &Vec<String> {
        match &self.path {
            LogSource::LogFile(_) => unimplemented!(
                "LogSource::LogList not permitted here in {} !",
                module_path!()
            ),
            LogSource::LogList(list) => list,
        }
    }

    // /// Return the list variant from LogSource
    // pub fn set_path(&mut self, path: &PathBuf) {
    //     self.path = LogSource::LogFile(path.clone());
    // }
}

/// A custom deserializer for the `exclude` field.
fn to_regex<'de, D>(deserializer: D) -> Result<Option<Regex>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Value = Deserialize::deserialize(deserializer)?;
    //println!("v= {:?}", v);
    let re = Regex::new(v.as_str().unwrap()).map_err(de::Error::custom)?;
    Ok(Some(re))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logfiledef() {
        let mut yaml = r#"
path: /var/log/syslog
format: json
exclude: "^error"
archive: 
    extension: xz
"#;
        let lfd: LogFileDef = serde_yaml::from_str(yaml).expect("unable to read YAML");
        assert_eq!(lfd.path(), &PathBuf::from("/var/log/syslog"));
        assert_eq!(lfd.format, LogFileFormat::json);
        assert_eq!(lfd.exclude.as_ref().unwrap().as_str(), "^error");
        assert_eq!(lfd.archive_path(), PathBuf::from("/var/log/syslog.xz"));

        yaml = r#"
path: /var/log/syslog
        "#;
        let lfd: LogFileDef = serde_yaml::from_str(yaml).expect("unable to read YAML");
        assert_eq!(lfd.path(), &PathBuf::from("/var/log/syslog"));
        assert_eq!(lfd.format, LogFileFormat::plain);
        assert!(lfd.exclude.is_none());
        assert_eq!(lfd.archive_path(), PathBuf::from("/var/log/syslog.1"));

        // test with a regex error
        yaml = r#"
path: /var/log/syslog
format: json
exclude: "^(error"
        "#;
        let lfd: Result<LogFileDef, serde_yaml::Error> = serde_yaml::from_str(yaml);
        assert!(lfd.is_err());

        // test with no regex
        yaml = r#"
path: /var/log/syslog
format: json
exclude: "^(error"
        "#;
        let lfd: Result<LogFileDef, serde_yaml::Error> = serde_yaml::from_str(yaml);
        assert!(lfd.is_err());

        // test with no regex
        yaml = r#"
list: 
    - /var/log
    - /tmp
format: json
        "#;
        let lfd: LogFileDef = serde_yaml::from_str(yaml).expect("unable to read YAML");
        assert_eq!(lfd.list(), &["/var/log", "/tmp"]);
        //assert_eq!(lfd.list(), &vec!["/var/log".to_string(), "/tmp".to_string()]);
        assert_eq!(lfd.format, LogFileFormat::json);
        assert!(lfd.exclude.is_none());
    }
}
