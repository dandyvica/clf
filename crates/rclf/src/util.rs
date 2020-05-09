//! Utility traits or structs.
use std::path::PathBuf;

/// Tells whether a `PathBuf` is accessible.
pub trait Usable {
    fn is_usable(&self) -> bool;
}

impl Usable for PathBuf {
    /// Tells whether a `PathBuf` is accessible i.e. it combines `has_root()`, `exists()` and `is_file()`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::path::PathBuf;
    /// use rclf::util::Usable;
    ///
    /// assert!(!PathBuf::from("foo.txt").is_usable());
    /// assert!(!PathBuf::from("/var/log/foo.txt").is_usable());
    /// assert!(!PathBuf::from("/var/log").is_usable());
    /// assert!(PathBuf::from("/var/log/syslog").is_usable());
    /// ```    
    fn is_usable(&self) -> bool {
        self.has_root() && self.exists() && self.is_file()
    }
}
