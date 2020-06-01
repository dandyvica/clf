//! A repository for all runtime logfile searches.
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use log::debug;
use serde::{Deserialize, Serialize};

use crate::{error::AppError, logfile::LogFile};

/// This structure will keep all run time information for each logfile searched. This is
/// a kind of central repository for all searches.
#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    //last_run:
    snapshot: HashMap<PathBuf, LogFile>,
}

impl Snapshot {
    /// Creates an empty snapshot.
    pub fn new() -> Snapshot {
        Snapshot {
            snapshot: HashMap::new(),
        }
    }

    /// Returns a default snapshot file name if not specified in the configuration file.
    pub fn default_name() -> PathBuf {
        let mut snapfile = std::env::temp_dir();
        snapfile.push("snapshot.json");
        snapfile
    }

    /// Deserialize a snapshot from a JSON file.
    pub fn load<P: AsRef<Path>>(snapshot_file: P) -> Result<Snapshot, AppError> {
        // open file, and create a new one if not found
        let json_file = match File::open(snapshot_file) {
            Ok(file) => file,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    return Ok(Snapshot::new());
                } else {
                    return Err(AppError::Io(e));
                }
            }
        };

        let reader = BufReader::new(json_file);

        // deserialize JSON
        let snapshot: Snapshot = serde_json::from_reader(reader)?;
        Ok(snapshot)
    }

    /// Serialize snapshot data to a JSON file.
    pub fn save<P: AsRef<Path>>(
        &mut self,
        snapshot_file: P,
        snapshot_retention: u64,
    ) -> Result<(), AppError> {
        // get number of seconds
        let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;

        // first delete tags having run before retention
        debug!("checking retention time for snapshot");
        //for (_, logfile) in &mut self.snapshot {
        for logfile in self.snapshot.values_mut() {
            let rundata = logfile.get_mut_rundata();
            // logfile
            //     .rundata
            //     .retain(|_, v| time.as_secs() - v.get_lastrun() < snapshot_retention);
            rundata.retain(|_, v| time.as_secs() - v.get_lastrun() < snapshot_retention);
        }

        // then just saves this file.
        let json_file = File::create(snapshot_file)?;
        serde_json::to_writer_pretty(json_file, self)?;

        Ok(())
    }

    /// Ensures a value is in the entry by inserting a value if empty, and returns a
    /// mutable reference to the value in the entry.
    // pub fn or_insert<P: AsRef<Path>>(&mut self, path: P) -> Result<&mut LogFile, AppError> {
    //     // is logfile already in the snapshot ?
    //     if !self.snapshot.contains_key(path.as_ref()) {
    //         // create a new LogFile
    //         let logfile = LogFile::new(&path)?;
    //         println!("logfile={:?}", logfile);
    //         let opt = self
    //             .snapshot
    //             .insert(path.as_ref().clone().to_path_buf(), logfile);
    //         debug_assert!(opt.is_none());
    //         debug_assert!(self.snapshot.contains_key(path.as_ref()));
    //     }
    //     debug_assert!(self.snapshot.contains_key(path.as_ref()));
    //     debug_assert!(self.snapshot.get_mut(path.as_ref()).is_some());

    //     Ok(self.snapshot.get_mut(path.as_ref()).unwrap())
    // }
    pub fn or_insert(&mut self, path: &PathBuf) -> Result<&mut LogFile, AppError> {
        // is logfile already in the snapshot ?
        if !self.snapshot.contains_key(path) {
            // create a new LogFile
            let logfile = LogFile::new(&path)?;
            println!("logfile={:?}", logfile);
            let opt = self.snapshot.insert(path.clone().to_path_buf(), logfile);
            debug_assert!(opt.is_none());
            debug_assert!(self.snapshot.contains_key(path));
        }
        debug_assert!(self.snapshot.contains_key(path));
        debug_assert!(self.snapshot.get_mut(path).is_some());

        Ok(self.snapshot.get_mut(path).unwrap())
    }

    /// Removes an entry in the snapshot.
    pub fn remove(&mut self, key: &PathBuf) -> Option<LogFile> {
        self.snapshot.remove(key)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    //use serde::{Deserialize, Serialize};

    // useful set of data for our unit tests
    const JSON: &'static str = r#"
    {
        "snapshot": {
            "/usr/bin/zip": {
                "path": "/usr/bin/zip",
                "compressed": false, 
                "inode": 1,
                "dev": 1,
                "rundata": {
                    "tag1": {
                        "name": "tag1",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000
                    },
                    "tag2": {
                        "name": "tag2",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000
                    }
                }
            },
            "/etc/hosts.allow": {
                "path": "/etc/hosts.allow",
                "compressed": false, 
                "inode": 1,
                "dev": 1,
                "rundata": {
                    "tag3": {
                        "name": "tag3",
                        "last_offset": 1000,
                        "last_line": 10,
                        "last_run": 1000000
                    },
                    "tag4": {
                        "name": "tag3",
                        "last_offset": 1500,
                        "last_line": 10,
                        "last_run": 1000000
                    }
                }
            }
        }
    }
    "#;

    #[test]
    fn load() {
        let data: Snapshot = serde_json::from_str(JSON).unwrap();

        let keys: Vec<_> = data.snapshot.keys().collect();

        assert!(keys.contains(&&PathBuf::from("/usr/bin/zip")));
        assert!(keys.contains(&&PathBuf::from("/etc/hosts.allow")));
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn or_insert() {
        let mut data: Snapshot = serde_json::from_str(JSON).unwrap();
        assert!(data.snapshot.contains_key(&PathBuf::from("/usr/bin/zip")));
        assert!(data
            .snapshot
            .contains_key(&PathBuf::from("/etc/hosts.allow")));
        assert_eq!(data.snapshot.len(), 2);
        // assert!(keys.contains(&&std::path::PathBuf::from("/etc/hosts.allow")));

        let mut logfile = data.or_insert(&PathBuf::from("/bin/gzip"));

        // snapshot has now 3 logfiles
        assert!(data.snapshot.contains_key(&PathBuf::from("/bin/gzip")));
        assert_eq!(data.snapshot.len(), 3);

        logfile = data.or_insert(&PathBuf::from("/usr/bin/zip"));
        assert_eq!(data.snapshot.len(), 3);
    }
}
