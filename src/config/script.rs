//! Contains the configuration of a script meant to be called either at the beginning of the search, for every line or at the end of all searches.
use std::process::Command;

use serde::Deserialize;

/// A callable script.
#[derive(Debug, Deserialize, Clone)]
pub struct Script {
    // command with its arguments
    pub command: Vec<String>,

    // optionnally, a timeout to wait for before moving on
    pub timeout: Option<u64>,
}

impl Script {
    /// Run command and optionnally wait for timeout
    pub fn spawn(&self) -> std::io::Result<u32> {
        let cmd = &self.command[0];
        let args = &self.command[1..];
        let result = Command::new(cmd).args(args).spawn()?;

        // now it's safe to unwrap to get pid
        let pid = result.id();

        // wait if specified
        if self.timeout.is_some() {
            let timeout = self.timeout.unwrap();
            let wait_timeout = std::time::Duration::from_millis(timeout);
            info!("prescript timeout specified, waiting {} ms", timeout);
            std::thread::sleep(wait_timeout);
        }

        info!("prescript command successfully executed, pid={}", pid);

        Ok(pid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    #[cfg(target_family = "unix")]
    fn spawn() {
        let mut script = Script {
            command: vec!["/usr/bin/find".to_string(), "/tmp".to_string()],
            timeout: Some(1000),
        };

        let pid = script.spawn();

        script.command = vec!["/usr/bin/fin".to_string(), ".".to_string()];
        let res = script.spawn();
        assert!(res.is_err());
    }
}
