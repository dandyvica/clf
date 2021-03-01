//! Holds the whole configuration data, loaded from a YAML file.
//!
//! This YAML file is divided into 2 parts:
//!
//! * a `global` YAML structure mapping the `Global` Rust structure which holds options which apply for each search
//! * an array of searches (the `searches`) tag which describes which files to search for, and the patterns which might
//! trigger a match.
//!
//! The logfile could either be an accessible file path, or a command which will be executed and gets back a list of files.
use std::path::Path;

use log::debug;
use serde::{de, Deserialize, Deserializer};
use serde_yaml::Value;

use super::{global::GlobalOptions, logsource::LogSource, search::Search};

use crate::misc::{
    error::{AppError, AppResult},
    extension::ListFiles,
};

use crate::{context, fromstr};

/// The main search configuration used to search patterns in a logfile. This is loaded from
/// the YAML file found in the command line argument (or from stdin). This configuration can include a list
/// of logfiles (given either by name or by starting an external command) to lookup and for each logfile, a list of regexes to match.
#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// List of global options, which apply for all searches.
    #[serde(default = "GlobalOptions::default")]
    pub global: GlobalOptions,

    /// list of searches.
    #[serde(deserialize_with = "fill_logdef")]
    pub searches: Vec<Search>,
}

// Auto-implement FromStr
fromstr!(Config);

impl Config {
    /// Loads a YAML configuration file as a `Config` struct, Tera version
    #[cfg(feature = "tera")]
    pub fn from_path<P: AsRef<Path> + std::fmt::Debug>(
        file_name: P,
        context: Option<&str>,
        show_rendered: bool,
    ) -> AppResult<Config> {
        use tera::{Context, Tera, Value};

        // read the whole file into a string
        let config = std::fs::read_to_string(&file_name)
            .map_err(|e| context!(e, "unable to read configuration file: {:?}", &file_name))?;

        // load context or create context if specified from arguments
        let context = if let Some(ctx) = context {
            let json: Value = serde_json::from_str(ctx)
                .map_err(|e| context!(e, "unable to context from JSON string {}", ctx))?;

            // create context from JSON string
            Context::from_value(json).expect("unable to add context")
        } else {
            Context::new()
        };

        // render the config with Tera context
        let rendered = Tera::one_off(&config, &context, false).expect("error one_off");
        if show_rendered {
            println!("{}", rendered);
            std::process::exit(0);
        }

        // load YAML data
        let yaml: Config = serde_yaml::from_str(&rendered)
            .map_err(|e| context!(e, "error in reading configuration file {:?}", file_name))?;

        debug!(
            "sucessfully loaded YAML configuration file, nb_searches={}",
            yaml.searches.len()
        );
        Ok(yaml)
    }

    /// Loads a YAML configuration file as a `Config` struct. Not using Tera
    #[cfg(not(feature = "tera"))]
    pub fn from_path<P: AsRef<Path> + std::fmt::Debug>(file_name: P) -> AppResult<Config> {
        // open YAML file
        let file = std::fs::File::open(&file_name)
            .map_err(|e| context!(e, "unable to read configuration file: {:?}", &file_name))?;

        // load YAML data
        let yaml: Config = serde_yaml::from_reader(file)
            .map_err(|e| context!(e, "error reading configuration file {:?}", file_name))?;
        debug!(
            "sucessfully loaded YAML configuration file, nb_searches={}",
            yaml.searches.len()
        );

        Ok(yaml)
    }
}

/// Replace the `logsource` YAML tag with the result of the script command
fn fill_logdef<'de, D>(deserializer: D) -> Result<Vec<Search>, D::Error>
where
    D: Deserializer<'de>,
{
    // get the YAML `Value` from serde. See https://docs.serde.rs/serde_yaml/enum.Value.html
    let yaml_value: Value = Deserialize::deserialize(deserializer)?;

    // transform this value into our struct
    let vec_yaml: Result<Vec<Search>, _> =
        serde_yaml::from_value(yaml_value).map_err(de::Error::custom);
    if vec_yaml.is_err() {
        return vec_yaml;
    }
    let mut vec_search = vec_yaml.unwrap();

    // this vector wil hold new logfiles from the list returned from the script execution
    let mut vec_loglist: Vec<Search> = Vec::new();

    for search in &vec_search {
        match &search.logfile.path {
            // we found a logfile tag: just copy everything to the new structure
            LogSource::LogFile(_) => continue,

            // we found a logslist tag: get the list of files, and for each one, copy everything
            LogSource::LogList(cmd) => {
                // get list of files from command or script
                let files = cmd.get_file_list().map_err(de::Error::custom)?;

                // create Search structure with the files we found, and a clone of all tags
                for file in &files {
                    // clone search structure
                    let mut cloned_search = search.clone();

                    // assign file instead of list
                    cloned_search.logfile.path = LogSource::LogFile(file.to_path_buf());

                    // now use this structure and add it to config_pathbuf
                    vec_loglist.push(cloned_search);
                }
            }

            // we found a logcommand tag: get the list of files using bash -c or cmd.exe /B, and for each one, copy everything
            LogSource::LogCommand(cmd) => {
                // get list of files from command or script
                let files = cmd.get_file_list().map_err(de::Error::custom)?;

                // create Search structure with the files we found, and a clone of all tags
                for file in &files {
                    // clone search structure
                    let mut cloned_search = search.clone();

                    // assign file instead of list
                    cloned_search.logfile.path = LogSource::LogFile(file.to_path_buf());

                    // now use this structure and add it to config_pathbuf
                    vec_loglist.push(cloned_search);
                }
            }
        }
    }

    // add all those new logfiles we found
    vec_search.extend(vec_loglist);

    // keep only valid logfiles, not logsources
    vec_search.retain(|x| x.logfile.path.is_path());
    Ok(vec_search)
}

#[cfg(test)]
mod tests {
    #[cfg(target_family = "unix")]
    use super::*;
    use std::path::PathBuf;

    #[test]
    #[cfg(target_family = "unix")]
    fn config() {
        dbg!(std::env::current_dir().unwrap());
        let yaml = r#"
        global:
          script_path: "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
          output_dir: /tmp/foo
          snapshot_file: /tmp/my_snapshot.json
          snapshot_retention: 5
          vars:
            first_name: Al
            last_name: Pacino
            city: 'Los Angeles'
            profession: actor
        
        searches:
          - logfile: 
                path: tests/logfiles/small_access.log
                format: plain
                exclude: '^error'
                archive: 
                    dir: /var/log
                    extension: gz
            tags: 
              - name: http_access_get_or_post
                process: true
                options: "warningthreshold=0"
                callback: { script: "tests/callbacks/echovars.py", args: ['arg1', 'arg2', 'arg3'] }
                patterns:
                  critical: {
                    regexes: [
                      'GET\s+([/\w]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)',
                    ],
                  }
                  warning: {
                    regexes: [
                      'POST\s+([/\w\.]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)'
                    ],
                    exceptions: [
                      '^\d{2,3}\.'
                    ]
                  }

          - logfile: 
                list: ['/usr/bin/find', '/var/log', '-type', '-f']
                format: plain
                exclude: '^error'
                archive: 
                    dir: /var/log
                    extension: gz
            tags: 
              - name: http_access_get_or_post
                process: true
                options: "warningthreshold=0"
                callback: { script: "tests/callbacks/echovars.py", args: ['arg1', 'arg2', 'arg3'] }
                patterns:
                  critical: {
                    regexes: [
                      'GET\s+([/\w]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)',
                    ],
                  }
                  warning: {
                    regexes: [
                      'POST\s+([/\w\.]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)'
                    ],
                    exceptions: [
                      '^\d{2,3}\.'
                    ]
                  }        
        "#;
        let config: Config = serde_yaml::from_str(yaml).expect("unable to read YAML");

        assert_eq!(
            &config.global.script_path,
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
        );
        assert_eq!(config.global.output_dir, PathBuf::from("/tmp/foo"));
        assert_eq!(
            config.global.snapshot_file,
            Some(PathBuf::from("/tmp/my_snapshot.json"))
        );
        assert_eq!(config.global.snapshot_retention, 5);
        assert_eq!(&config.global.global_vars.get("first_name").unwrap(), &"Al");
        assert_eq!(
            &config.global.global_vars.get("last_name").unwrap(),
            &"Pacino"
        );
        assert_eq!(
            &config.global.global_vars.get("city").unwrap(),
            &"Los Angeles"
        );
        assert_eq!(
            &config.global.global_vars.get("profession").unwrap(),
            &"actor"
        );
        assert_eq!(config.searches.len(), 1);

        let search = config.searches.first().unwrap();
        assert_eq!(
            search.logfile.path(),
            &PathBuf::from("tests/logfiles/small_access.log")
        );
        assert_eq!(search.tags.len(), 1);

        let tag = search.tags.first().unwrap();
        assert_eq!(&tag.name, "http_access_get_or_post");
        assert!(tag.process);
        assert_eq!(tag.options.warningthreshold, 0);
        assert!(tag.callback.is_some());
        let script = PathBuf::from("tests/callbacks/echovars.py");
        assert!(
            matches!(&tag.callback.as_ref().unwrap().callback, crate::configuration::callback::CallbackType::Script(Some(x)) if x == &script)
        );
        assert_eq!(
            tag.callback.as_ref().unwrap().args.as_ref().unwrap(),
            &["arg1", "arg2", "arg3"]
        );
        assert!(tag.patterns.ok.is_none());
        assert!(tag.patterns.critical.is_some());
        assert!(tag.patterns.warning.is_some());
    }
}
