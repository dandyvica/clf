use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, ErrorKind};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{error::AppError, logfile::LogFile};

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    //last_run:
    rundata: HashMap<PathBuf, LogFile>,
}

impl Snapshot {
    pub fn new() -> Snapshot {
        Snapshot {
            rundata: HashMap::new(),
        }
    }

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

    pub fn save<P: AsRef<Path>>(&self, snapshot_file: P) -> Result<(), AppError> {
        let json_file = File::create(snapshot_file)?;
        serde_json::to_writer_pretty(json_file, self)?;
        Ok(())
    }

    // get a mutable reference
    pub fn get_mut_or_insert(&mut self, path: &Path) -> Result<&mut LogFile, AppError> {
        // is logfile already in the snapshot ?
        if !self.rundata.contains_key(path) {
            // create a new LogFile
            let logfile = LogFile::new(path)?;
            let opt = self.rundata.insert(path.clone().to_path_buf(), logfile);
            debug_assert!(opt.is_none());
        }

        Ok(self.rundata.get_mut(path).unwrap())
    }

    pub fn insert(&mut self, logfile_name: &Path, logfile_data: LogFile) -> Option<LogFile> {
        self.rundata
            .insert(logfile_name.clone().to_path_buf(), logfile_data)
    }
}
