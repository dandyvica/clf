//! Contains the configuration of a script meant to be called either at the beginning of the search, for every line or at the end of all searches.
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::configuration::vars::GlobalVars;
use crate::misc::{nagios::Nagios, util::DEFAULT_SCRIPT_TIMEOUT};

/// A callable script.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Script {
    /// command with its arguments
    pub command: Vec<String>,

    /// optionally, a timeout to wait for before moving on
    #[serde(default = "Script::default_timeout")]
    pub timeout: u64,

    /// if async is set, spawn the process and don't wait
    #[serde(rename = "async")]
    #[serde(default)]
    pub async_flag: bool,

    /// exit clf with UNKNOW if script exit code is non 0
    #[serde(default)]
    pub exit_on_error: bool,
}

impl Script {
    // default timeout for prescript waiting
    pub fn default_timeout() -> u64 {
        DEFAULT_SCRIPT_TIMEOUT
    }

    /// Run command and optionnally wait for timeout
    pub fn spawn(&self, vars: Option<&GlobalVars>) -> std::io::Result<u32> {
        let cmd = &self.command[0];
        let args = &self.command[1..];
        trace!("script is called with arguments: {:?}", &self.command);

        // optionally use args to start the script
        let mut child = match vars {
            None => Command::new(cmd)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?,
            Some(vars) => {
                trace!("script is called with extra vars: {:?}", vars);
                Command::new(cmd)
                    .envs(vars)
                    .args(args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?
            }
        };

        // now it's safe to unwrap to get pid
        let pid = child.id();
        info!("script:{:?} started, pid:{}", &self.command, pid);

        // wait for timeout
        self.sleep();

        // if async, don't wait and just leave
        if self.async_flag {
            trace!("async flag set, returning with pid:{}", pid);
            return Ok(pid);
        }

        // try to get the exit status
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() && self.exit_on_error {
                    #[cfg(not(test))]
                    Nagios::exit_unknown(&format!(
                        "script:{:?}, pid:{}, exit_on_error is set and exit code is:{} ",
                        &self.command,
                        pid,
                        status.code().unwrap()
                    ));
                    #[cfg(test)]
                    std::process::exit(0);
                } else {
                    info!(
                        "script:{:?}, pid:{}, exit code is:{} ",
                        &self.command,
                        pid,
                        status.code().unwrap()
                    );
                    #[cfg(test)]
                    std::process::exit(0);
                }
            }
            Ok(None) => {
                let result = child.kill();
                info!(
                    "script:{:?}, pid:{}, timeout occured, pid kill() result={:?}",
                    &self.command, pid, result
                );
                #[cfg(not(test))]
                Nagios::exit_unknown(&format!(
                    "script: {:?} timed-out, pid:{}",
                    &self.command, pid
                ))
            }
            Err(e) => Nagios::exit_unknown(&format!(
                "script {:?} couldn't start, error={} !",
                self.command, e
            )),
        }

        let output = child.wait_with_output().expect("failed to wait on child");
        info!(
            "stdout={:?}, stderr={:?}",
            std::str::from_utf8(&output.stdout),
            std::str::from_utf8(&output.stderr)
        );
        Ok(pid)
    }

    // just sleep main thread with specified timeout
    fn sleep(&self) {
        let timeout = self.timeout;
        if timeout != 0 {
            let wait_timeout = std::time::Duration::from_millis(timeout);
            info!("script timeout specified, waiting {} ms", timeout);
            std::thread::sleep(wait_timeout);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_family = "unix")]
    fn spawn() {
        // async
        let yaml = r#"
command: ["/usr/bin/find", "/tmp"]
async: true   
"#;

        let script: Script = serde_yaml::from_str(yaml).expect("unable to read YAML");
        let _pid = script.spawn(None);

        // sync timeout = 5s
        let yaml = r#"
command: ["sleep", "1000"]
timeout: 2000   
"#;

        let script: Script = serde_yaml::from_str(yaml).expect("unable to read YAML");
        let _pid = script.spawn(None);

        // sync timeout = 5s
        let yaml = r#"
command: ["date", "'%f'"]
timeout: 100  
exit_on_error: true 
"#;

        let script: Script = serde_yaml::from_str(yaml).expect("unable to read YAML");
        let _pid = script.spawn(None);
    }

    #[test]
    #[cfg(target_family = "windows")]
    fn spawn() {
        // async
        let yaml = r#"
command: ["cmd.exe", "/c", "dir c:\\windows\\system32 | findstr windows"]
async: true   
"#;

        let script: Script = serde_yaml::from_str(yaml).expect("unable to read YAML");
        let _pid = script.spawn(None);

        // sync timeout = 5s
        let yaml = r#"
command: ["attrib"]
timeout: 2000   
"#;

        let script: Script = serde_yaml::from_str(yaml).expect("unable to read YAML");
        let _pid = script.spawn(None);

        // sync timeout = 5s
        let yaml = r#"
command: ["ping", "www.google.com", "-n", "4"]
timeout: 6000  
exit_on_error: true 
"#;

        let script: Script = serde_yaml::from_str(yaml).expect("unable to read YAML");
        let _pid = script.spawn(None);
    }
}
