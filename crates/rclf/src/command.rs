//! Useful wrapper on the `Command` standard lib structure.
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;
use std::thread;

use log::{debug, info};
use serde::Deserialize;
use wait_timeout::ChildExt;

/// Returns the number of seconds for a standard timeout when not defined in the YAML file.
/// Neede by `serde`.
const fn default_timeout() -> u64 {
    2 * 3600
}

use crate::error::AppError;
//use crate::error::AppError;
#[derive(Debug, Deserialize)]
pub struct Cmd {
    /// The name of the script/command to start.
    path: PathBuf,

    /// Option arguments of the previous.
    args: Option<Vec<String>>,

    /// A timeout in seconds to for wait command completion.
    #[serde(default = "self::default_timeout")]
    timeout: u64,
}

impl Cmd {
    /// This spawns a command and expects a list of file names.
    pub fn get_list<P: AsRef<OsStr>>(cmd: P, args: &[&str]) -> Result<Vec<PathBuf>, AppError> {
        let output = Command::new(&cmd).args(args).output()?;
        debug!("output={:?}", output);
        let output_as_str = std::str::from_utf8(&output.stdout)?;

        Ok(output_as_str
            .lines()
            .map(|s| PathBuf::from(s))
            .collect::<Vec<PathBuf>>())
    }

    /// Spawns the script, and wait at most `timeout` seconds for the job to finish. Updates the PATH
    /// environment variable before spawning the command.
    pub fn spawn(&self, env_path: Option<&str>) -> Result<thread::JoinHandle<()>, AppError> {
        let timeout = self.timeout;
        let mut cmd = Command::new(&self.path);
        let mut child = match env_path {
            None => cmd.args(&self.args.as_ref().unwrap()[..]).spawn()?,
            Some(_env_path) => cmd
                .args(&self.args.as_ref().unwrap()[..])
                .env("PATH", _env_path)
                .spawn()?,
        };

        // spawns a new thread to not be blocked waiting for command to finish
        let handle = thread::spawn(move || {
            let duration = std::time::Duration::from_secs(timeout);
            let _status_code = match child.wait_timeout(duration).unwrap() {
                Some(status) => status.code(),
                None => {
                    // child hasn't exited yet
                    child.kill().unwrap();
                    child.wait().unwrap().code()
                }
            };
        });
        Ok(handle)
    }
}
