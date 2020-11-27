//! A repository for all runtime logfile searches.
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use log::debug;
use serde::{Deserialize, Serialize};

use crate::logfile::logfile::LogFile;
use crate::misc::error::AppError;

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
    pub fn load<P: AsRef<Path>>(snapshot_file: P) -> Result<Snapshot, AppError> {
        // open file, and create a new one if not found
        let json_file = match File::open(snapshot_file) {
            Ok(file) => file,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    return Ok(Snapshot::default());
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
        for logfile in self.snapshot.values_mut() {
            let run_data = logfile.rundata_mut();
            run_data.retain(|_, v| time.as_secs() - v.lastrun_secs() < snapshot_retention);
        }

        // then just saves this file.
        let json_file = File::create(snapshot_file)?;
        serde_json::to_writer_pretty(json_file, self)?;

        Ok(())
    }

    /// Creates a new `LogfiFile` struct if not found, or retrieve an already stored one in
    /// the snapshot.
    pub fn logfile_mut(&mut self, path: &PathBuf) -> Result<&mut LogFile, AppError> {
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

        Ok(self.snapshot.get_mut(path).unwrap())
    }

    /// Builds a default snapshot file name.
    pub fn default_name() -> PathBuf {
        let mut snapfile = std::env::temp_dir();
        snapfile.push("clf_snapshot.json");
        snapfile
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    use crate::testing::data::*;

    #[test]
    fn load() {
        let data: Snapshot = serde_json::from_str(SNAPSHOT_SAMPLE).unwrap();

        let keys: Vec<_> = data.snapshot.keys().collect();

        assert!(keys.contains(&&PathBuf::from("/usr/bin/zip")));
        assert!(keys.contains(&&PathBuf::from("/etc/hosts.allow")));
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn logfile_mut() {
        let mut data: Snapshot = serde_json::from_str(SNAPSHOT_SAMPLE).unwrap();
        assert!(data.snapshot.contains_key(&PathBuf::from("/usr/bin/zip")));
        assert!(data
            .snapshot
            .contains_key(&PathBuf::from("/etc/hosts.allow")));
        assert_eq!(data.snapshot.len(), 2);
        // assert!(keys.contains(&&std::path::PathBuf::from("/etc/hosts.allow")));

        let _ = data.logfile_mut(&PathBuf::from("/bin/gzip"));

        // snapshot has now 3 logfiles
        assert!(data.snapshot.contains_key(&PathBuf::from("/bin/gzip")));
        assert_eq!(data.snapshot.len(), 3);

        let _ = data.logfile_mut(&PathBuf::from("/usr/bin/zip"));
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
