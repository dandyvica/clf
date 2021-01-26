//! Contains the global configuration when processing logfiles. These values are independant from the ones solely related to a logfile when searching.
use std::borrow::Cow;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::configuration::{script::Script, vars::GlobalVars};
use crate::misc::constants::*;

use crate::{fromstr, prefix_var};
#[derive(Debug, Deserialize, Clone)]
/// A list of global options, which apply globally for all searches.
#[serde(default)]
pub struct GlobalOptions {
    /// A list of paths, separated by either ':' for unix, or ';' for Windows. This is
    /// where the script, if any, will be searched for. Default to PATH or Path depending on the platform.
    pub script_path: String,

    /// A directory where matched lines will be stored.
    pub output_dir: PathBuf,

    /// The snapshot file name. Option<> is used because if not specified here,
    pub snapshot_file: Option<PathBuf>,

    /// Retention time for tags.
    pub snapshot_retention: u64,

    /// A list of user variables if any.
    #[serde(rename = "vars")]
    pub global_vars: GlobalVars,

    // A command called before starting reading
    pub prescript: Option<Vec<Script>>,

    // A command called before the end of clf
    pub postscript: Option<Script>,
}

impl GlobalOptions {
    /// Add variables like user, platform etc not dependant from a logfile
    pub fn insert_process_env<P: AsRef<Path>>(&mut self, path: P) {
        // add config file name
        self.global_vars.insert_var(
            prefix_var!("CONFIG_FILE"),
            path.as_ref().to_string_lossy().to_string(),
        );

        // now just add variables
        self.global_vars
            .insert_var(prefix_var!("USER"), whoami::username());
        self.global_vars
            .insert_var(prefix_var!("HOSTNAME"), whoami::hostname());
        self.global_vars
            .insert_var(prefix_var!("PLATFORM"), whoami::platform().to_string());
    }

    /// Add optional extra global variables coming from the command line
    pub fn insert_extra_vars(&mut self, vars: &Option<Vec<String>>) {
        if vars.is_some() {
            let vars = vars.as_ref().unwrap();

            // each var should have this form: var=value
            for var in vars {
                // split at char ':'
                let splitted: Vec<&str> = var.split(':').collect();

                // if we don't find the equals sign just loop
                if splitted.len() != 2 {
                    continue;
                }

                // now it's safe to insert
                self.global_vars
                    .insert_var(prefix_var!(splitted[0]), splitted[1].to_string());
            }
        }
    }
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
            script_path: path_var,
            output_dir: std::env::temp_dir(),
            snapshot_file: None,
            snapshot_retention: DEFAULT_RETENTION,
            global_vars: GlobalVars::default(),
            prescript: None,
            postscript: None,
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
script_path: /usr/foo1
snapshot_file: /usr/foo3/snap.foo
output_dir: /usr/foo2
        "#;

        let mut opts = GlobalOptions::from_str(yaml).expect("unable to read YAML");
        //println!("opts={:?}", opts);

        assert_eq!(&opts.script_path, "/usr/foo1");
        assert_eq!(opts.output_dir, PathBuf::from("/usr/foo2"));
        assert_eq!(
            opts.snapshot_file,
            Some(PathBuf::from("/usr/foo3/snap.foo"))
        );

        yaml = r#"
script_path: /usr/foo1

# a list of user variables, if any
vars:
    first_name: Al
    last_name: Pacino
    city: 'Los Angeles'
    profession: actor            
        "#;

        opts = GlobalOptions::from_str(yaml).expect("unable to read YAML");
        assert_eq!(&opts.script_path, "/usr/foo1");
        assert_eq!(opts.output_dir, PathBuf::from("/tmp"));
        assert_eq!(opts.snapshot_file, None);

        let vars = opts.global_vars;
        assert_eq!(vars.get("first_name").unwrap(), "Al");
        assert_eq!(vars.get("last_name").unwrap(), "Pacino");
        assert_eq!(vars.get("city").unwrap(), "Los Angeles");
        assert_eq!(vars.get("profession").unwrap(), "actor");
    }
}
