//! Contains the configuration of the archiving process of a logfile. WWe can define here how, where and the naming convention
//! of an archived file that has been rotated, usually using `logrotate` UNIX process.
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// This structure keeps everything related to log rotations
#[derive(Debug, Deserialize, Clone)]
pub struct LogArchive {
    /// the logfile name to check
    dir: Option<PathBuf>,

    /// the most recent archived path
    archive: Option<PathBuf>,

    /// a regex pattern to determine which archive to get
    pattern: Option<String>,
}

impl LogArchive {
    /// When no archive is specified, just get the standard logrotate file name: add .1 at the end of the logfile
    pub fn rotated_path<P: AsRef<Path> + Clone>(path: P) -> PathBuf {
        // build the file name by appending .1 to its path
        let rotated_path = format!("{}.1", path.as_ref().to_string_lossy());

        PathBuf::from(rotated_path)
    }

    // When a LogArchive struct is specified in the config file, build the archive file name
    pub fn archived_path<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        // build the directory for the archived path
        let dir = match &self.dir {
            None => {
                // extract path from original path. It's safe to unwrap() because all paths should be absolute
                let dir = path.as_ref().parent();
                debug_assert!(dir.is_some());
                dir.unwrap()
            }
            Some(dir) => &dir,
        };
        debug_assert!(dir.is_dir());

        // if not archive is specified, just add .1 at the end of the path
        let rotated_path = if self.archive.is_none() {
            format!("{}.1", path.as_ref().to_string_lossy())
        } else {
            format!(
                "{}/{}",
                dir.to_string_lossy(),
                self.archive.as_ref().unwrap().to_string_lossy()
            )
        };

        PathBuf::from(rotated_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_family = "unix")]
    fn rotated_path() {
        let mut p = PathBuf::from("/var/log/kern.log");
        assert_eq!(
            LogArchive::rotated_path(p),
            PathBuf::from("/var/log/kern.log.1")
        );

        p = PathBuf::from("/var/log/syslog");
        assert_eq!(
            LogArchive::rotated_path(p),
            PathBuf::from("/var/log/syslog.1")
        );
    }

    //#[test]
    #[cfg(target_family = "unix")]
    fn archived_path() {
        let mut p = PathBuf::from("/var/log/kern.log");

        let mut archive = LogArchive {
            dir: None,
            archive: None,
            pattern: None,
        };
        assert_eq!(
            archive.archived_path(&p),
            PathBuf::from("/var/log/kern.log.1")
        );

        let mut archive = LogArchive {
            dir: Some(PathBuf::from("/tmp")),
            archive: None,
            pattern: None,
        };
        assert_eq!(archive.archived_path(&p), PathBuf::from("/tmp/kern.log.1"));

        // let mut archive = LogArchive {
        //     dir: None,
        //     archive: None,
        //     pattern: None,
        // };
        // assert_eq!(
        //     archive.archived_path(&p),
        //     PathBuf::from("/var/log/kern.log.1")
        // );
    }
}
