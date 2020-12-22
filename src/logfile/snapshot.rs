//! A repository for all runtime logfile searches.
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use log::debug;
use serde::{Deserialize, Serialize};

use crate::config::{logfiledef::LogFileDef, pattern::PatternCounters};
use crate::context;
use crate::logfile::{logfile::LogFile, logfileerror::LogFileAccessErrorList};
use crate::misc::{
    error::{AppError, AppResult},
    nagios::{NagiosError, NagiosExit},
};

/// This structure will keep all run time information for each logfile searched. This is
/// a kind of central repository for all searches.
#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    // snapshot file name
    //#[serde(skip)]
    //path: PathBuf,

    //last_run:
    snapshot: HashMap<PathBuf, LogFile>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Snapshot {
            snapshot: HashMap::new(),
        }
    }
}

impl Snapshot {
    /// Builds a new snapshot file name from `path`.
    pub fn build_name(path: &PathBuf) -> PathBuf {
        // get file name from path variable
        let name = path.file_stem().unwrap_or(OsStr::new("clf_snapshot"));

        // builds new name
        let mut snapshot_file = PathBuf::new();

        snapshot_file.push(name);
        snapshot_file.set_extension("json");

        snapshot_file
    }

    /// Deserialize a snapshot from a JSON file.
    pub fn load<P: AsRef<Path> + Debug>(snapshot_file: P) -> AppResult<Snapshot> {
        // open file, and create a new one if not found
        let json_file = match File::open(&snapshot_file) {
            Ok(file) => file,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    return Ok(Snapshot::default());
                } else {
                    return Err(AppError::from_error(
                        e,
                        &format!("error loading snapshot file: {:?}", snapshot_file),
                    ));
                }
            }
        };

        let reader = BufReader::new(json_file);

        // deserialize JSON
        let snapshot: Snapshot = serde_json::from_reader(reader)
            .map_err(|e| context!(e, "unable load snapshot file: {:?}", snapshot_file))?;
        Ok(snapshot)
    }

    /// Serialize snapshot data to a JSON file.
    pub fn save<P: AsRef<Path> + Debug>(
        &mut self,
        snapshot_file: P,
        snapshot_retention: u64,
    ) -> AppResult<()> {
        // get number of seconds
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| context!(e, "duration_since() error: {:?}", snapshot_file))?;

        // first delete tags having run before retention
        debug!("checking retention time for snapshot");
        for logfile in self.snapshot.values_mut() {
            let run_data = logfile.rundata_mut();
            run_data.retain(|_, v| time.as_secs() - v.last_run_secs < snapshot_retention);
        }

        // then just saves this file.
        let json_file = File::create(&snapshot_file)
            .map_err(|e| context!(e, "unable create snapshot file: {:?}", snapshot_file))?;
        serde_json::to_writer_pretty(json_file, self)
            .map_err(|e| context!(e, "to_writer_pretty() error",))?;

        Ok(())
    }

    /// Creates a new `LogfiFile` struct if not found, or retrieve an already stored one in
    /// the snapshot.
    pub fn logfile_mut(&mut self, path: &PathBuf, def: &LogFileDef) -> AppResult<&mut LogFile> {
        // is logfile already in the snapshot ?
        if !self.snapshot.contains_key(path) {
            // create a new LogFile
            let logfile = LogFile::from_path(&path)?;
            let opt = self.snapshot.insert(path.clone().to_path_buf(), logfile);
            debug_assert!(opt.is_none());
            debug_assert!(self.snapshot.contains_key(path));
        }
        debug_assert!(self.snapshot.contains_key(path));
        debug_assert!(self.snapshot.get_mut(path).is_some());

        // get element mutable ref and set missing fields
        let logfile = self.snapshot.get_mut(path).unwrap();
        logfile.set_definition(def.clone());

        Ok(logfile)
    }

    /// Builds a default snapshot file name.
    pub fn default_path() -> PathBuf {
        let mut snapfile = std::env::temp_dir();
        snapfile.push("clf_snapshot.json");
        snapfile
    }

    /// Builds the final output message displayed by the plugin
    pub fn exit_message(&self, access_errors: &LogFileAccessErrorList) -> NagiosError {
        // calculate the summation of all pattern counts for all logfiles
        let pattern_sum = self
            .snapshot
            .values() // Vec<LogFile>
            .map(|x| x.sum_counters()) // Vec<PatternCounters>
            .fold(PatternCounters::default(), |acc, x| acc + x); // PatternCounters

        // build the nagios exit counters
        let mut global_exit = NagiosExit::default();
        global_exit.critical_count = pattern_sum.critical_count;
        global_exit.warning_count = pattern_sum.warning_count;

        // unknown is a special case: we sum the number of cases where an error occurred in RunData structures
        for logfile in self.snapshot.values() {
            global_exit.unknown_count += logfile
                .run_data
                .values()
                .filter(|x| x.last_error.is_some())
                .count() as u64;
        }

        let nagios_error = NagiosError::from(&global_exit);
        println!("{}", global_exit);

        // loop through all run data
        for (path, logfile) in &self.snapshot {
            for (tag_name, run_data) in &logfile.run_data {
                if run_data.pid == std::process::id() {
                    let nagios_exit = NagiosExit::from(run_data);
                    println!("{}(tag={}) - {}", path.display(), tag_name, nagios_exit);
                }
            }
        }

        // then list access errors
        for (path, access_error) in access_errors.iter() {
            println!(
                "{} - {}: {}",
                path.display(),
                String::from(&access_error.nagios_error),
                access_error.error
            );
        }

        nagios_error
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    const SNAPSHOT_SAMPLE: &'static str = r#"
    {
        "snapshot": {
            "/var/log/kern.log": {
              "path": "/var/log/kern.log",
              "directory": "/var/log",
              "extension": "log",
              "compression": "uncompressed",
              "signature": {
                "inode": 1104686,
                "dev": 28
              },
              "run_data": {
                "kern_kernel": {
                  "last_offset": 4556,
                  "last_line": 42,
                  "last_run": "2020-11-29 18:20:11.660118341",
                  "last_run_secs": 1606674011,
                  "counters": {
                    "critical_count": 0,
                    "warning_count": 42,
                    "ok_count": 0,
                    "exec_count": 10
                  },
                  "last_error": "None"
                },
                "kern_nokernel": {
                  "last_offset": 122,
                  "last_line": 1,
                  "last_run": "2020-11-29 18:20:11.627811908",
                  "last_run_secs": 1606674011,
                  "counters": {
                    "critical_count": 0,
                    "warning_count": 1,
                    "ok_count": 0,
                    "exec_count": 0
                  },
                  "last_error": "No such file or directory (os error 2)"
                }
              }
            }
          }
     }
 "#;

    #[test]
    fn load() {
        let data: Snapshot = serde_json::from_str(SNAPSHOT_SAMPLE).unwrap();
        let keys: Vec<_> = data.snapshot.keys().collect();
        assert!(keys.contains(&&PathBuf::from("/var/log/kern.log")));
    }

    //#[test]
    #[cfg(target_family = "unix")]
    fn logfile_mut() {
        let mut data: Snapshot = serde_json::from_str(SNAPSHOT_SAMPLE).unwrap();
        let def = LogFileDef::default();

        assert!(data
            .snapshot
            .contains_key(&PathBuf::from("/var/log/kern.log")));
        assert!(data
            .snapshot
            .contains_key(&PathBuf::from("/etc/hosts.allow")));
        assert_eq!(data.snapshot.len(), 2);

        let _ = data.logfile_mut(&PathBuf::from("/bin/gzip"), &def);

        // snapshot has now 3 logfiles
        assert!(data.snapshot.contains_key(&PathBuf::from("/bin/gzip")));
        assert_eq!(data.snapshot.len(), 3);

        let _ = data.logfile_mut(&PathBuf::from("/usr/bin/zip"), &def);
        assert_eq!(data.snapshot.len(), 3);
    }

    // #[test]
    // #[cfg(target_family = "unix")]
    // fn from_path() {
    //     let config = PathBuf::from("/home/johndoe/config.yml");

    //     assert_eq!(
    //         Snapshot::from_path(&config),
    //         PathBuf::from("/tmp/config.json")
    //     );
    //     assert_eq!(
    //         Snapshot::from_path(&config, Some(PathBuf::from("/home/foo"))),
    //         PathBuf::from("/home/foo/config.json")
    //     );
    // }
}
