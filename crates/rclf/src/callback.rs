//! Useful wrapper on the `Command` Rust standard library structure.
use std::ffi::OsStr;
use std::io::Write;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Instant;

use log::{debug, info};
use serde::Deserialize;

/// Returns the number of seconds for a standard timeout when not defined in the YAML file.
/// Neede by `serde`.
const fn default_timeout() -> u64 {
    2 * 3600
}

use crate::{error::AppError, variables::Variables};

/// A callback could be either synchronous, or asynchronous.
#[derive(Debug, Deserialize, PartialEq, Hash, Eq)]
#[allow(non_camel_case_types)]
pub enum CallbackType {
    synchronous,
    asynchronous,
}

/// Represents the class of callback: script to be called, ABI to be run, ip:port address to send, ...
#[derive(Debug, Deserialize, PartialEq, Hash, Eq)]
#[allow(non_camel_case_types)]
pub enum CallbackClass {
    script,
    tcp,
}

/// A structure representing a command to start
#[derive(Debug, Deserialize, Clone)]
pub struct Callback {
    /// The name of the script/command to start if defined.
    pub path: Option<PathBuf>,

    /// The address:port of the remote or local script to which sent the data, as JSON.
    pub address: Option<String>,

    /// Option arguments of the previous.
    args: Option<Vec<String>>,

    /// A timeout in seconds to for wait command completion.
    #[serde(default = "self::default_timeout")]
    timeout: u64,
}

impl Callback {
    /// This spawns a command and expects a list of file names.
    pub fn get_list<S: AsRef<OsStr>>(
        cmd: S,
        args: Option<&[String]>,
    ) -> Result<Vec<PathBuf>, AppError> {
        let output = match args {
            None => Command::new(&cmd).output()?,
            Some(_args) => Command::new(&cmd).args(_args).output()?,
        };

        debug!("output={:?}", output);
        let output_as_str = std::str::from_utf8(&output.stdout)?;

        Ok(output_as_str
            .lines()
            .map(PathBuf::from)
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
    pub fn spawn(&self, env_path: Option<&str>, vars: &Variables) -> Result<ChildData, AppError> {
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
        debug_assert!(self.path.is_some());
        let mut cmd = Command::new(&self.path.as_ref().unwrap());

        // runtime variables are always there.
        cmd.envs(&vars.runtime_vars);

        // user variables, maybe
        if let Some(uservars) = &vars.user_vars {
            cmd.envs(uservars);
        }

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

        Ok(ChildData {
            child: Some(child),
            path: self.path.as_ref().unwrap().clone(),
            timeout: self.timeout,
            start_time: Some(Instant::now()),
        })
    }

    /// Sends all variables, as a JSON string, to the specified address:port.
    pub fn send(&self, vars: &Variables) -> Result<(), AppError> {
        debug_assert!(self.address.is_some());
        let mut stream = TcpStream::connect(&self.address.as_ref().unwrap())?;

        debug!(
            "sending JOSN data to address: {}",
            &self.address.as_ref().unwrap()
        );
        let json = vars.to_json();
        stream.write(&json.as_bytes())?;

        Ok(())
    }
}

/// Return structure from a call to a script. Gathers all relevant data, instead of a mere tuple.
#[derive(Debug, Default)]
pub struct ChildData {
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
    fn list_files_find() {
        let files = Callback::get_list(
            &"find",
            Some(&[
                "/var/log".to_string(),
                "-ctime".to_string(),
                "+1".to_string(),
            ]),
        )
        .expect("error listing files");
        assert!(files.len() > 10);
        assert!(files.iter().all(|f| f.starts_with("/var/log")));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn list_files_ls() {
        let files = Callback::get_list(
            &"bash",
            Some(&["-c".to_string(), "ls /var/log/*.log".to_string()]),
        )
        .expect("error listing files");
        assert!(files.len() > 10);
        assert!(files.iter().all(|f| f.starts_with("/var/log")));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn list_files_shell() {
        let files = Callback::get_list(
            &"cmd.exe",
            Some(&[
                "/c".to_string(),
                r"dir /b c:\windows\system32\*.dll".to_string(),
            ]),
        )
        .expect("error listing files");
        //println!("{:?}", files);
        assert!(files.len() > 1000);
        //assert!(files.iter().all(|f| f.ends_with("dll")));
    }
}
