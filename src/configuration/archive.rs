//! Contains the configuration of the archiving process of a logfile. WWe can define here how, where and the naming convention
//! of an archived file that has been rotated, usually using `logrotate` UNIX process.
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use serde::Deserialize;

/// This structure keeps everything related to log rotations
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LogArchive {
    /// the logfile name to check
    pub dir: Option<PathBuf>,

    /// the most recent archived path
    pub extension: Option<String>,

    /// a regex pattern to determine which archive to get
    pub pattern: Option<String>,
}

impl LogArchive {
    /// When no archive is specified, just get the standard logrotate file name: add .1 at the end of the logfile
    pub fn default_path<P: AsRef<Path> + Clone>(path: P) -> PathBuf {
        // build the file name by appending .1 to its path
        let default_path = format!("{}.1", path.as_ref().to_string_lossy());

        PathBuf::from(default_path)
    }

    // When a LogArchive struct is specified in the config file, build the archive file name
    pub fn archived_path<P: AsRef<Path> + std::fmt::Debug>(&self, path: P) -> PathBuf {
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
        println!("dir={:?}", dir);

        // extract file name
        debug_assert!(path.as_ref().file_name().is_some());
        let file_name = path.as_ref().file_name().unwrap().to_string_lossy();

        // if not archive is specified, just add .1 at the end of the path
        #[cfg(target_family = "windows")]
        let default_path = if self.extension.is_none() {
            format!("{}\\{}.1", dir.to_string_lossy(), file_name)
        } else {
            format!(
                "{}\\{}.{}",
                dir.to_string_lossy(),
                file_name,
                self.extension.as_ref().unwrap()
            )
        };

        // if not archive is specified, just add .1 at the end of the path
        #[cfg(target_family = "unix")]
        let default_path = if self.extension.is_none() {
            format!("{}/{}.1", dir.to_string_lossy(), file_name)
        } else {
            format!(
                "{}/{}.{}",
                dir.to_string_lossy(),
                file_name,
                self.extension.as_ref().unwrap()
            )
        };

        println!(
            "self={:?}, dir={}, file={}, rotated={}",
            path,
            dir.display(),
            file_name,
            default_path
        );

        PathBuf::from(default_path)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    #[cfg(target_family = "unix")]
    fn default_path() {
        let yaml = r#"
dir: /var/log
extension: xz
"#;
        let archive: LogArchive = serde_yaml::from_str(yaml).expect("unable to read YAML");
        println!("{:#?}", archive);

        let mut p = PathBuf::from("/var/log/kern.log");
        assert_eq!(
            LogArchive::default_path(p),
            PathBuf::from("/var/log/kern.log.1")
        );

        p = PathBuf::from("/var/log/syslog");
        assert_eq!(
            LogArchive::default_path(p),
            PathBuf::from("/var/log/syslog.1")
        );
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn archived_path() {
        //
        let p = PathBuf::from("/var/log/kern.log");

        let archive = LogArchive {
            dir: None,
            extension: None,
            pattern: None,
        };
        assert_eq!(
            archive.archived_path(&p),
            PathBuf::from("/var/log/kern.log.1")
        );

        //
        let yaml = r#"
dir: /tmp
"#;
        let archive: LogArchive = serde_yaml::from_str(yaml).expect("unable to read YAML");
        assert_eq!(archive.archived_path(&p), PathBuf::from("/tmp/kern.log.1"));

        //
        let yaml = r#"
extension: gz
"#;
        let archive: LogArchive = serde_yaml::from_str(yaml).expect("unable to read YAML");
        assert_eq!(
            archive.archived_path(&p),
            PathBuf::from("/var/log/kern.log.gz")
        );

        //
        let yaml = r#"
dir: /tmp
extension: gz
"#;
        let archive: LogArchive = serde_yaml::from_str(yaml).expect("unable to read YAML");
        assert_eq!(archive.archived_path(&p), PathBuf::from("/tmp/kern.log.gz"));
    }

    #[test]
    #[cfg(target_family = "windows")]
    fn archived_path() {
        let p = PathBuf::from(r"C:\Windows\WindowsUpdate.log");

        let archive = LogArchive {
            dir: None,
            extension: None,
            pattern: None,
        };
        assert_eq!(
            archive.archived_path(&p),
            PathBuf::from(r"C:\Windows\WindowsUpdate.log.1")
        );

        let archive = LogArchive {
            dir: Some(PathBuf::from(r"c:\Windows\Temp")),
            extension: None,
            pattern: None,
        };
        assert_eq!(
            archive.archived_path(&p),
            PathBuf::from(r"C:\Windows\Temp\WindowsUpdate.log.1")
        );

        let archive = LogArchive {
            dir: None,
            extension: Some("gz".to_string()),
            pattern: None,
        };
        assert_eq!(
            archive.archived_path(&p),
            PathBuf::from(r"C:\Windows\WindowsUpdate.log.gz")
        );

        let archive = LogArchive {
            dir: Some(PathBuf::from(r"c:\Windows\Temp")),
            extension: Some("gz".to_string()),
            pattern: None,
        };
        assert_eq!(
            archive.archived_path(&p),
            PathBuf::from(r"c:\Windows\Temp\WindowsUpdate.log.gz")
        );
    }
}
