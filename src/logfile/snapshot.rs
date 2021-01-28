//! A repository for all runtime logfile searches. These values are kept as a JSON file and reused each time the process is run.
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufReader, ErrorKind};
use std::path::{Path, PathBuf};

use log::debug;
use serde::{Deserialize, Serialize};

use crate::configuration::{logfiledef::LogFileDef, pattern::PatternCounters};
use crate::context;
use crate::logfile::{logfile::LogFile, logfileerror::LogFileAccessErrorList};
use crate::misc::{
    error::{AppError, AppResult},
    nagios::{NagiosError, NagiosExit},
    util::from_epoch_secs,
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
        let name = path
            .file_stem()
            .unwrap_or_else(|| OsStr::new("clf_snapshot"));

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
        let seconds_from_epoch = from_epoch_secs()?;

        // first delete tags having run before retention
        debug!("checking retention time for snapshot");
        for logfile in self.snapshot.values_mut() {
            let run_data = logfile.rundata_mut();
            run_data.retain(|_, v| seconds_from_epoch - v.last_run_secs < snapshot_retention);
        }

        // because of before deletion, some logfiles might not include run_data anymore. So no need to keep them
        self.snapshot.retain(|_, v| !v.run_data.is_empty());

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
            trace!(
                "snapshot is not containing path {:?}, creating a new entry",
                path
            );
            let logfile = LogFile::from_path(&path, Some(def.clone()))?;
            let opt = self.snapshot.insert(path.clone(), logfile);
            debug_assert!(opt.is_none());
            debug_assert!(self.snapshot.contains_key(path));
        }
        debug_assert!(self.snapshot.contains_key(path));
        debug_assert!(self.snapshot.get_mut(path).is_some());

        // get element mutable ref and set missing fields
        let logfile = self.snapshot.get_mut(path).unwrap();
        logfile.set_definition(def.clone());

        trace!("created logfile struct: {:#?}", logfile);

        Ok(logfile)
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

        // add critical, warning or unknown error count with access errors
        for (_, access_error) in access_errors.iter() {
            match access_error.nagios_error {
                NagiosError::CRITICAL => global_exit.critical_count += 1,
                NagiosError::WARNING => global_exit.warning_count += 1,
                NagiosError::UNKNOWN => global_exit.unknown_count += 1,
                NagiosError::OK => (),
            }
        }

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
        "/var/log/apt/term.log": {
        "id": {
            "declared_path": "/var/log/apt/term.log",
            "canon_path": "/var/log/apt/term.log",
            "directory": "/var/log/apt",
            "extension": "log",
            "compression": "uncompressed",
            "signature": {
                "inode": 1275587,
                "dev": 28,
                "size": 4000
            }
        },
        "run_data": {
            "apt": {
            "pid": 40468,
            "start_offset": 197326,
            "start_line": 999,
            "last_offset": 98607,
            "last_line": 1100,
            "last_run": "2020-12-22 16:10:55.286912679",
            "last_run_secs": 1611857382,
            "counters": {
                "critical_count": 0,
                "warning_count": 5,
                "ok_count": 0,
                "exec_count": 5
            },
            "last_error": "None"
            }
        }
        },
        "/var/log/apt/history.log": {
        "id": {
            "declared_path": "/var/log/apt/history.log",
            "canon_path": "/var/log/apt/history.log",
            "directory": "/var/log/apt",
            "extension": "log",
            "compression": "uncompressed",
            "signature": {
                "inode": 1275587,
                "dev": 28,
                "size": 4000
            }
        },
        "run_data": {
            "apt": {
            "pid": 40468,
            "start_offset": 197326,
            "start_line": 999,
            "last_offset": 13996,
            "last_line": 68,
            "last_run": "2020-12-22 16:10:55.287475585",
            "last_run_secs": 1611857382,
            "counters": {
                "critical_count": 0,
                "warning_count": 0,
                "ok_count": 0,
                "exec_count": 0
            },
            "last_error": "None"
            }
        }
        },
        "/var/log/kern.log": {
        "id": {
            "declared_path": "/var/log/kern.log",
            "canon_path": "/var/log/kern.log",
            "directory": "/var/log",
            "extension": "log",
            "compression": "uncompressed",
            "signature": {
                "inode": 1275587,
                "dev": 28,
                "size": 4000
            }
        },
        "run_data": {
            "kern_kernel": {
            "pid": 40468,
            "start_offset": 197326,
            "start_line": 999,
            "last_offset": 392201,
            "last_line": 3885,
            "last_run": "2020-12-22 16:10:55.280019283",
            "last_run_secs": 1611857382,
            "counters": {
                "critical_count": 0,
                "warning_count": 3885,
                "ok_count": 0,
                "exec_count": 10
            },
            "last_error": "None"
            },
            "kern_nokernel": {
            "pid": 40468,
            "start_offset": 197326,
            "start_line": 999,
            "last_offset": 392201,
            "last_line": 3885,
            "last_run": "2020-12-22 16:10:54.102239131",
            "last_run_secs": 1611857382,
            "counters": {
                "critical_count": 0,
                "warning_count": 3867,
                "ok_count": 0,
                "exec_count": 3867
            },
            "last_error": "None"
            }
        }
        },
        "/var/log/syslog": {
        "id": {
            "declared_path": "/var/log/syslog",
            "canon_path": "/var/log/syslog",
            "directory": "/var/log",
            "extension": null,
            "compression": "uncompressed",
            "signature": {
                "inode": 1275587,
                "dev": 28,
                "size": 4000
            }
        },
        "run_data": {
            "syslog_nokernel": {
            "pid": 40468,
            "start_offset": 197326,
            "start_line": 999,
            "last_offset": 334147,
            "last_line": 3152,
            "last_run": "2020-12-22 16:10:48.877119302",
            "last_run_secs": 1611857382,
            "counters": {
                "critical_count": 0,
                "warning_count": 0,
                "ok_count": 0,
                "exec_count": 0
            },
            "last_error": "None"
            },
            "syslog_kernel": {
            "pid": 40468,
            "start_offset": 197326,
            "start_line": 999,
            "last_offset": 334147,
            "last_line": 3152,
            "last_run": "2020-12-22 16:10:51.412855148",
            "last_run_secs": 1611857382,
            "counters": {
                "critical_count": 0,
                "warning_count": 1400,
                "ok_count": 0,
                "exec_count": 1400
            },
            "last_error": "None"
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
        assert!(keys.contains(&&PathBuf::from("/var/log/syslog")));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn logfile_mut() {
        let mut data: Snapshot = serde_json::from_str(SNAPSHOT_SAMPLE).unwrap();
        let mut def = LogFileDef::default();
        def.hash_window = 4096;

        assert!(data
            .snapshot
            .contains_key(&PathBuf::from("/var/log/kern.log")));
        assert!(data
            .snapshot
            .contains_key(&PathBuf::from("/var/log/syslog")));
        assert_eq!(data.snapshot.len(), 4);

        let _ = data.logfile_mut(&PathBuf::from("/bin/gzip"), &def);

        // snapshot has now 3 logfiles
        assert!(data.snapshot.contains_key(&PathBuf::from("/bin/gzip")));
        assert_eq!(data.snapshot.len(), 5);

        let _ = data.logfile_mut(&PathBuf::from("/usr/bin/zip"), &def);
        assert_eq!(data.snapshot.len(), 6);
    }
}
