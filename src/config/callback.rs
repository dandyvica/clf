//! Useful wrapper on the `Command` Rust standard library structure.
use std::cell::RefCell;
use std::convert::TryFrom;
use std::io::Write;
use std::net::TcpStream;

#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;

use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Instant;

use log::{debug, info};
use serde::Deserialize;
use serde_json::json;

use crate::config::variables::Variables;
use crate::fromstr;
use crate::misc::{error::AppError, util::Cons};

/// A callback is either a script, or a TCP socket or a UNIX domain socket
#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone)]
pub enum CallbackType {
    #[serde(rename = "script")]
    Script(Option<PathBuf>),

    #[serde(rename = "address")]
    Tcp(Option<String>),

    #[serde(rename = "domain")]
    Domain(Option<PathBuf>),
}

/// Represent a TCP or UNIX socket
#[derive(Debug, Default)]
pub struct CallbackHandle {
    cmd: Option<Command>,
    tcp_socket: Option<TcpStream>,
    domain_socket: Option<UnixStream>,
}

/// A fake implementation because TcpStream etc don't implement Clone
impl Clone for CallbackHandle {
    fn clone(&self) -> Self {
        CallbackHandle {
            cmd: None,
            tcp_socket: None,
            domain_socket: None,
        }
    }
}

/// A structure representing a command to start
#[derive(Debug, Deserialize, Clone)]
pub struct Callback {
    /// A callback identifier is either a script path, a TCP socket or a UNIX domain socket
    #[serde(flatten)]
    pub(in crate) callback: CallbackType,

    /// Option arguments of the previous.
    pub(in crate) args: Option<Vec<String>>,

    /// A timeout in seconds to for wait command completion.
    #[serde(default = "Cons::default_timeout")]
    timeout: u64,
}

impl Callback {
    /// Calls the relevant callback with arguments
    pub fn call(
        &self,
        env_path: Option<&str>,
        vars: &Variables,
        handle: &mut CallbackHandle,
    ) -> Result<Option<ChildData>, AppError> {
        debug!(
            "ready to start callback {:?} with args={:?}, path={:?}, envs={:?}, current_dir={:?}",
            &self.callback,
            self.args,
            env_path,
            vars,
            std::env::current_dir()
                .map_err(|e| format!("unable to fetch current directory, error={}", e))
        );

        // the callback is called depending of its type
        match &self.callback {
            CallbackType::Script(path) => {
                // build Command struct before execution.
                debug_assert!(path.is_some());

                let mut cmd = Command::new(path.as_ref().unwrap());

                // user vars don't change so we can add them right now
                if let Some(uservars) = vars.user_vars() {
                    cmd.envs(uservars);
                }

                // add arguments if any
                if let Some(args) = &self.args {
                    cmd.args(&args[..]);
                }

                //handle.cmd = Some(cmd);
                debug!("creating Command for: {:?}", path.as_ref().unwrap());

                // runtime variables are always there.
                cmd.envs(vars.runtime_vars());

                // update PATH variable if any
                if let Some(path) = env_path {
                    cmd.env("PATH", path);
                }

                // start command
                let child = cmd.spawn()?;
                info!("starting script {:?}, pid={}", path, child.id());

                Ok(Some(ChildData {
                    child: Some(RefCell::new(child)),
                    path: path.as_ref().unwrap().clone(),
                    timeout: self.timeout,
                    start_time: Some(Instant::now()),
                }))
            }
            CallbackType::Tcp(address) => {
                // test whether a TCP socket is already created
                if handle.tcp_socket.is_none() {
                    let stream = TcpStream::connect(address.as_ref().unwrap())?;
                    handle.tcp_socket = Some(stream);
                    debug!("creating TCP socket for: {}", address.as_ref().unwrap());
                }

                // send JSON data through TCP socket
                let mut stream = handle.tcp_socket.as_ref().unwrap();

                // create a dedicated JSON structure
                let mut json = json!({
                    "args": &self.args,
                    "vars": vars
                })
                .to_string();

                // 64KB a payload is more than enough
                json.truncate(u16::MAX as usize);
                let json_raw = json.as_bytes();

                // send data length first in network order, and then send payload
                let size = u16::try_from(json_raw.len()).expect(&format!(
                    "unexpected conversion error at {}-{}",
                    file!(),
                    line!()
                ));
                stream.write(&size.to_be_bytes())?;
                stream.write(&json.as_bytes())?;

                Ok(None)
            }
            CallbackType::Domain(address) => {
                // test whether a UNIX socket is already created
                if handle.domain_socket.is_none() {
                    let stream = UnixStream::connect(address.as_ref().unwrap())?;
                    handle.domain_socket = Some(stream);
                    debug!("creating UNIX socket for: {:?}", address.as_ref().unwrap());
                }

                // send JSON data through UNIX socket
                let mut stream = handle.domain_socket.as_ref().unwrap();

                // create a dedicated JSON structure
                let mut json = json!({
                    "args": &self.args,
                    "vars": vars
                })
                .to_string();

                // 64KB a payload is more than enough
                json.truncate(u16::MAX as usize);
                let json_raw = json.as_bytes();

                // send data length first in network order, and then send payload
                let size = u16::try_from(json_raw.len()).expect(&format!(
                    "unexpected conversion error at {}-{}",
                    file!(),
                    line!()
                ));
                stream.write(&size.to_be_bytes())?;
                //println!("size={:?}", &json_raw.len().to_be_bytes());
                stream.write(&json.as_bytes())?;

                Ok(None)
            }
        }
    }
}

// Auto-implement FromStr
fromstr!(Callback);

/// Return structure from a call to a script. Gathers all relevant data, instead of a mere tuple.
#[derive(Debug, Default)]
pub struct ChildData {
    pub child: Option<RefCell<Child>>,
    pub path: PathBuf,
    pub timeout: u64,
    pub start_time: Option<Instant>,
}

impl ChildData {
    #[cfg(test)]
    fn exit_code(&mut self) -> Result<Option<i32>, AppError> {
        // do we have a Child ?
        if self.child.is_none() {
            return Ok(None);
        }

        // now it's safe to unwrap
        let child = &mut self.child.as_ref().unwrap().borrow_mut();
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status.code()),
            Ok(None) => {
                let res = child.wait();
                return Ok(res.unwrap().code());
            }
            Err(e) => return Err(crate::misc::error::AppError::Io(e)),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    //use regex::Regex;
    use std::io::prelude::*;
    use std::str::FromStr;

    use crate::config::variables::Variables;
    //use crate::misc::error::AppError;
    use crate::testing::data::sample_vars;

    #[derive(Deserialize)]
    struct JSON {
        args: Vec<String>,
        vars: Variables,
    }

    // utility fn to receive JSON from a stream
    fn get_json<T: Read>(socket: &mut T) -> JSON {
        // read size first
        let mut size_buffer = [0; std::mem::size_of::<u16>()];
        socket.read_exact(&mut size_buffer).unwrap();
        let json_size = u16::from_be_bytes(size_buffer);
        //assert_eq!(json_size, 211);

        // read JSON raw data
        let mut json_buffer = vec![0; json_size as usize];
        socket.read_exact(&mut json_buffer).unwrap();

        // get JSON
        let s = std::str::from_utf8(&json_buffer).unwrap();

        let json: JSON = serde_json::from_str(&s).unwrap();
        json
    }

    // helper fn to create a dummy Variables struct
    // fn dummy_vars() -> Variables {
    //     // create dummy variables
    //     let re = Regex::new(r"^([a-z\s]+) (\w+) (\w+) (?P<LASTNAME>\w+)").unwrap();
    //     let text = "my name is john fitzgerald kennedy, president of the USA";

    //     let mut vars = Variables::default();
    //     vars.insert_captures(&re, text);

    //     vars
    // }

    #[test]
    #[cfg(target_family = "unix")]
    fn callback_script() {
        let yaml = r#"
            script: "tests/scripts/check_ut.py"
            args: ['one', 'two', 'three']
        "#;

        let cb: Callback = Callback::from_str(yaml).expect("unable to read YAML");
        let script = PathBuf::from("tests/scripts/check_ut.py");
        assert!(matches!(&cb.callback, CallbackType::Script(Some(x)) if x == &script));
        assert_eq!(cb.args.as_ref().unwrap().len(), 3);

        // create dummy variables
        let vars = sample_vars();

        // call script
        let mut handle = CallbackHandle::default();
        let data = cb.call(None, &vars, &mut handle).unwrap();
        assert!(data.is_some());

        // safe to unwrap
        let mut child_data = data.unwrap();

        // get exit code from script
        let code = child_data.exit_code();
        assert!(code.is_ok());
        assert_eq!(code.unwrap(), Some(0));
    }

    #[test]
    fn callback_tcp() {
        let yaml = r#"
            address: 127.0.0.1:8900
            args: ['one', 'two', 'three']
        "#;

        let cb = Callback::from_str(yaml).expect("unable to read YAML");
        let addr = "127.0.0.1:8900".to_string();
        assert!(matches!(&cb.callback, CallbackType::Tcp(Some(x)) if x == &addr));

        // create a very simple TCP server: wait for data and test them
        let child = std::thread::spawn(move || {
            // create a listener
            let listener = std::net::TcpListener::bind(&addr).unwrap();
            match listener.accept() {
                Ok((mut socket, _addr)) => {
                    let json = get_json(&mut socket);

                    assert_eq!(json.args, vec!["one", "two", "three"]);

                    assert_eq!(json.vars.get("CLF_CAPTURE1").unwrap(), "my name is");
                    assert_eq!(json.vars.get("CLF_CAPTURE2").unwrap(), "john");
                    assert_eq!(json.vars.get("CLF_CAPTURE3").unwrap(), "fitzgerald");
                    assert_eq!(json.vars.get("CLF_LASTNAME").unwrap(), "kennedy");
                }
                Err(e) => panic!("couldn't get client: {:?}", e),
            }
        });

        // wait a little
        let ten_millis = std::time::Duration::from_millis(10);
        std::thread::sleep(ten_millis);

        // create dummy variables
        let vars = sample_vars();

        // some work here
        let mut handle = CallbackHandle::default();
        let data = cb.call(None, &vars, &mut handle).unwrap();
        assert!(data.is_none());

        let _res = child.join();
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn callback_domain() {
        let yaml = r#"
            domain: /tmp/callback.sock
            args: ['one', 'two', 'three']
        "#;

        let cb = Callback::from_str(yaml).expect("unable to read YAML");
        let addr = PathBuf::from("/tmp/callback.sock");

        let _ = std::fs::remove_file(&addr);

        assert!(matches!(&cb.callback, CallbackType::Domain(Some(x)) if x == &addr));

        // create a very simple UNIX socket server: wait for data and test them
        let child = std::thread::spawn(move || {
            // create a listener
            let listener = std::os::unix::net::UnixListener::bind(addr).unwrap();
            match listener.accept() {
                Ok((mut socket, _addr)) => {
                    let json = get_json(&mut socket);

                    assert_eq!(json.args, vec!["one", "two", "three"]);

                    assert_eq!(json.vars.get("CLF_CAPTURE1").unwrap(), "my name is");
                    assert_eq!(json.vars.get("CLF_CAPTURE2").unwrap(), "john");
                    assert_eq!(json.vars.get("CLF_CAPTURE3").unwrap(), "fitzgerald");
                    assert_eq!(json.vars.get("CLF_LASTNAME").unwrap(), "kennedy");
                }
                Err(e) => panic!("couldn't get client: {:?}", e),
            }
        });

        // wait a little
        let ten_millis = std::time::Duration::from_millis(10);
        std::thread::sleep(ten_millis);

        // create dummy variables
        let mut vars = sample_vars();

        // some work here
        let mut handle = CallbackHandle::default();
        let data = cb.call(None, &mut vars, &mut handle).unwrap();
        assert!(data.is_none());

        //cb.call(None, &vars).unwrap();

        let _res = child.join();
    }
}
