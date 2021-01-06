//! Contains the global configuration when processing logfiles. These values are independant from the ones solely related to a logfile when searching.
use std::path::PathBuf;

use serde::Deserialize;

use crate::config::{script::Script, vars::UserVars};
use crate::logfile::snapshot::Snapshot;
use crate::misc::constants::*;

use crate::fromstr;

#[derive(Debug, Deserialize, Clone)]
/// A list of global options, which apply globally for all searches.
#[serde(default)]
pub struct GlobalOptions {
    /// A list of paths, separated by either ':' for unix, or ';' for Windows. This is
    /// where the script, if any, will be searched for. Default to PATH or Path depending on the platform.
    pub path: String,

    /// A directory where matched lines will be stored.
    pub output_dir: PathBuf,

    /// The snapshot file name. Option<> is used because if not specified here,
    pub snapshot_file: Option<PathBuf>,

    /// Retention time for tags.
    pub snapshot_retention: u64,

    /// A list of user variables if any.
    pub user_vars: Option<UserVars>,

    // A command called before starting reading
    pub prescript: Option<Script>,

    // A command called before the end of clf
    pub postcript: Option<Script>,
}

// Auto-implement FromStr
fromstr!(GlobalOptions);

/// Default implementation, rather than serde default field attribute.
impl Default for GlobalOptions {
    fn default() -> Self {
        // default path
        let path_var = if cfg!(target_family = "unix") {
            std::env::var("PATH").unwrap_or_else(|_| "/usr/sbin:/usr/bin:/sbin:/bin".to_string())
        } else if cfg!(target_family = "windows") {
            std::env::var("Path").unwrap_or_else(|_| {
                r"C:\Windows\system32;C:\Windows;C:\Windows\System32\Wbem;".to_string()
            })
        } else {
            unimplemented!("unsupported OS, file: {}:{}", file!(), line!());
        };

        // default logger path
        let mut logger_path = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
        logger_path.push("clf.log");

        GlobalOptions {
            path: path_var,
            output_dir: std::env::temp_dir(),
            snapshot_file: None,
            snapshot_retention: DEFAULT_RETENTION,
            user_vars: None,
            prescript: None,
            postcript: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    #[cfg(target_family = "unix")]
    fn global_options() {
        let mut yaml = r#"
path: /usr/foo1
snapshot_file: /usr/foo3/snap.foo
output_dir: /usr/foo2
        "#;

        let mut opts = GlobalOptions::from_str(yaml).expect("unable to read YAML");
        //println!("opts={:?}", opts);

        assert_eq!(&opts.path, "/usr/foo1");
        assert_eq!(opts.output_dir, PathBuf::from("/usr/foo2"));
        assert_eq!(
            opts.snapshot_file,
            Some(PathBuf::from("/usr/foo3/snap.foo"))
        );

        yaml = r#"
path: /usr/foo1

# a list of user variables, if any
user_vars:
    first_name: Al
    last_name: Pacino
    city: 'Los Angeles'
    profession: actor            
        "#;

        opts = GlobalOptions::from_str(yaml).expect("unable to read YAML");
        assert_eq!(&opts.path, "/usr/foo1");
        assert_eq!(opts.output_dir, PathBuf::from("/tmp"));
        assert_eq!(opts.snapshot_file, None);
        assert!(opts.user_vars.is_some());

        let vars = opts.user_vars.unwrap();
        assert_eq!(vars.get("first_name").unwrap(), "Al");
        assert_eq!(vars.get("last_name").unwrap(), "Pacino");
        assert_eq!(vars.get("city").unwrap(), "Los Angeles");
        assert_eq!(vars.get("profession").unwrap(), "actor");
    }
}
