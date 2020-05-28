//! Utility traits or structs.
use std::path::PathBuf;

/// Tells whether a `PathBuf` is accessible.
pub trait Usable {
    fn is_usable(&self) -> bool;
}

impl Usable for PathBuf {
    /// Tells whether a `PathBuf` is accessible i.e. it combines `has_root()`, `exists()` and `is_file()`.  
    fn is_usable(&self) -> bool {
        //self.has_root() && self.exists() && self.is_file()
        self.exists() && self.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn is_usable() {
        assert!(!PathBuf::from("foo.txt").is_usable());
        assert!(!PathBuf::from("/var/log/foo.txt").is_usable());
        assert!(!PathBuf::from("/var/log").is_usable());
        assert!(PathBuf::from("/var/log/syslog").is_usable());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn is_usable() {
        assert!(!PathBuf::from("foo.txt").is_usable());
        assert!(!PathBuf::from(r"c:\windows\system32\foo.txt").is_usable());
        assert!(!PathBuf::from(r"c:\windows\system32").is_usable());
        assert!(PathBuf::from(r"c:\windows\system32\cmd.exe").is_usable());
    }
}
