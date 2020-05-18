//! Useful wrapper on the `Command` Rust standard library structure.
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Instant;

use log::{debug, error, info};
use serde::Deserialize;

/// Returns the number of seconds for a standard timeout when not defined in the YAML file.
/// Neede by `serde`.
const fn default_timeout() -> u64 {
    2 * 3600
}

use crate::{error::AppError, logfile::LookupRet, variables::Vars};
#[derive(Debug, Deserialize, Clone)]
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
    pub fn get_list<P, I>(cmd: P, args: I) -> Result<Vec<PathBuf>, AppError>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<OsStr>,
    {
        let output = Command::new(&cmd).args(args).output()?;
        debug!("output={:?}", output);
        let output_as_str = std::str::from_utf8(&output.stdout)?;

        Ok(output_as_str
            .lines()
            .map(|s| PathBuf::from(s))
            .collect::<Vec<PathBuf>>())
    }

    // /// This spawns a command and expects a list of file names.
    // pub fn call<P: AsRef<OsStr>>(cmd: P, args: &str) -> Result<Vec<PathBuf>, AppError> {
    //     // split out arguments
    //     let arg_list: Vec<_> = args.split_whitespace().collect();

    //     // create command and get output
    //     let output = Command::new(&cmd).args(arg_list).output()?;
    //     debug!("output={:?}", output);
    //     let output_as_str = std::str::from_utf8(&output.stdout)?;

    //     Ok(output_as_str
    //         .lines()
    //         .map(|s| PathBuf::from(s))
    //         .collect::<Vec<PathBuf>>())
    // }

    /// Spawns the script, and wait at most `timeout` seconds for the job to finish. Updates the PATH
    /// environment variable before spawning the command. Also add all variables as environment variables.
    pub fn spawn(&self, env_path: Option<&str>, vars: &Vars) -> LookupRet {
        debug!(
            "ready to start {:?} with args={:?}, path={:?}, envs={:?}, current_dir={:?}",
            self.path,
            self.args,
            env_path,
            vars,
            std::env::current_dir()
                .map_err(|e| format!("unable to fetch current directory, error={}", e))
        );

        // build Command struct before execution.
        let mut cmd = Command::new(&self.path);

        // variables are always there.
        cmd.envs(vars.inner());

        // update PATH variable if any
        if let Some(path) = env_path {
            cmd.env("PATH", path);
        }

        // add arguments if any
        if let Some(args) = &self.args {
            cmd.args(&args[..]);
        }

        // start command
        let child = cmd.spawn()?;
        info!("starting script {:?}, pid={}", self.path, child.id());

        Ok(ChildReturn {
            child: Some(child),
            path: self.path.clone(),
            timeout: self.timeout,
            start_time: Some(Instant::now()),
        })
    }
}

/// Return structure from a call to a script. Gathers all relevant data, instead of a mere tuple.
#[derive(Debug, Default)]
pub struct ChildReturn {
    pub child: Option<Child>,
    pub path: PathBuf,
    pub timeout: u64,
    pub start_time: Option<Instant>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn list_files_shell() {
        let files =
            Cmd::get_list(&"find", &["/var/log", "-ctime", "+1"]).expect("error listing files");
        assert!(files.len() > 10);
        assert!(files.iter().all(|f| f.starts_with("/var/log")));
    }
    #[test]
    #[cfg(target_os = "windows")]
    fn list_files_shell() {
        // let files = Cmd::get_list("find", &["/var/log", "-ctime", "+1"]).expect("error listing files");
        // assert!(files.len() > 10);
        // assert!(files.iter().all(|f| f.starts_with("/var/log")));
    }
}
