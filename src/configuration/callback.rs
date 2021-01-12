//! Contains the configuration of what is executed each time a pattern is found in the logfile. It could be either a spawned script, a TCP socket to which send
//! relevant data, or a Unix Datagram Socket. For the 2 latter cases, found data are sent as a JSON string. Otherwise, when a script is called, data are sent
//! through environment variables.
use std::borrow::Cow;
use std::cell::RefCell;
use std::convert::TryFrom;
use std::io::Write;
use std::net::TcpStream;

#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;

use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Instant;

use log::debug;
use serde::Deserialize;
use serde_json::json;

use crate::configuration::vars::{GlobalVars, RuntimeVars};
use crate::misc::{
    constants::*,
    error::{AppError, AppResult},
};
use crate::{context, fromstr};

/// A callback is either a script, or a TCP socket or a UNIX domain socket
#[derive(Debug, Deserialize, PartialEq, Hash, Eq, Clone)]
pub enum CallbackType {
    #[serde(rename = "script")]
    Script(Option<PathBuf>),

    #[serde(rename = "address")]
    Tcp(Option<String>),

    #[serde(rename = "domain")]
    #[cfg(target_family = "unix")]
    Domain(Option<PathBuf>),
}

/// Represent a TCP or UNIX socket
#[derive(Debug, Default)]
pub struct CallbackHandle {
    cmd: Option<Command>,
    tcp_socket: Option<TcpStream>,
    #[cfg(target_family = "unix")]
    domain_socket: Option<UnixStream>,
}

/// A fake implementation because TcpStream etc don't implement Clone
impl Clone for CallbackHandle {
    fn clone(&self) -> Self {
        CallbackHandle {
            cmd: None,
            tcp_socket: None,
            #[cfg(target_family = "unix")]
            domain_socket: None,
        }
    }
}

/// A structure representing a command to start
#[derive(Debug, Deserialize, Clone)]
pub struct Callback {
    /// A callback identifier is either a script path, a TCP socket or a UNIX domain socket
    #[serde(flatten)]
    pub callback: CallbackType,

    /// Option arguments of the previous.
    pub args: Option<Vec<String>>,

    /// A timeout in seconds to for wait command completion.
    #[serde(default = "default_timeout")]
    timeout: u64,
}

impl Callback {
    /// Calls the relevant callback with arguments
    pub fn call(
        &self,
        env_path: Option<&str>,
        global_vars: &GlobalVars,
        runtime_vars: &RuntimeVars,
        handle: &mut CallbackHandle,
    ) -> AppResult<Option<ChildData>> {
        debug!(
            "ready to start callback {:?} with args={:?}, path={:?}, envs={:?}, current_dir={:?}",
            &self.callback,
            self.args,
            env_path,
            runtime_vars,
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
                if global_vars.len() != 0 {
                    cmd.envs(global_vars.inner());
                }

                // add arguments if any
                if let Some(args) = &self.args {
                    cmd.args(&args[..]);
                }

                //handle.cmd = Some(cmd);
                debug!("creating Command for: {:?}", path.as_ref().unwrap());

                // runtime variables are always there.
                for (var, value) in runtime_vars.inner() {
                    match var {
                        Cow::Borrowed(s) => cmd.env(s, &value),
                        Cow::Owned(s) => cmd.env(s, &value),
                    };
                }

                // update PATH variable if any
                if let Some(path) = env_path {
                    cmd.env("PATH", path);
                }

                // start command
                let child = cmd
                    .spawn()
                    .map_err(|e| context!(e, "unable to spawn process for cmd:{:?}", path))?;
                debug!("starting script {:?}, pid={}", path, child.id());

                Ok(Some(ChildData {
                    child: Some(RefCell::new(child)),
                    path: path.as_ref().unwrap().clone(),
                    timeout: self.timeout,
                    start_time: Some(Instant::now()),
                }))
            }
            CallbackType::Tcp(address) => {
                debug_assert!(address.is_some());
                let addr = address.as_ref().unwrap();

                // test whether a TCP socket is already created
                if handle.tcp_socket.is_none() {
                    let stream = TcpStream::connect(addr)
                        .map_err(|e| context!(e, "unable to connect to TCP address: {}", addr))?;
                    handle.tcp_socket = Some(stream);
                    debug!("creating TCP socket for: {}", address.as_ref().unwrap());
                }

                // send JSON data through TCP socket
                let mut stream = handle.tcp_socket.as_ref().unwrap();

                // create a dedicated JSON structure
                let mut json = json!({
                    "args": &self.args,
                    "global": global_vars,
                    "vars": runtime_vars
                })
                .to_string();

                // 64KB a payload is more than enough
                json.truncate(u16::MAX as usize);
                let json_raw = json.as_bytes();

                // send data length first in network order, and then send payload
                let size = u16::try_from(json_raw.len()).unwrap_or_else(|_| {
                    panic!("unexpected conversion error at {}-{}", file!(), line!())
                });

                stream.write(&size.to_be_bytes()).map_err(|e| {
                    context!(
                        e,
                        "error writing payload size:{} to address: {:?}",
                        size,
                        addr
                    )
                })?;
                stream
                    .write(&json.as_bytes())
                    .map_err(|e| context!(e, "error writing JSON data to address: {:?}", addr))?;

                Ok(None)
            }
            #[cfg(target_family = "unix")]
            CallbackType::Domain(address) => {
                debug_assert!(address.is_some());
                let addr = address.as_ref().unwrap();

                // test whether a UNIX socket is already created
                if handle.domain_socket.is_none() {
                    let stream = UnixStream::connect(address.as_ref().unwrap()).map_err(|e| {
                        context!(e, "unable to connect to UNIX socket address: {:?}", addr)
                    })?;
                    handle.domain_socket = Some(stream);
                    debug!("creating UNIX socket for: {:?}", address.as_ref().unwrap());
                }

                // send JSON data through UNIX socket
                let mut stream = handle.domain_socket.as_ref().unwrap();

                // create a dedicated JSON structure
                let mut json = json!({
                    "args": &self.args,
                    "global": global_vars,
                    "vars": runtime_vars
                })
                .to_string();

                // 64KB a payload is more than enough
                json.truncate(u16::MAX as usize);
                let json_raw = json.as_bytes();

                // send data length first in network order, and then send payload
                let size = u16::try_from(json_raw.len()).unwrap_or_else(|_| {
                    panic!("unexpected conversion error at {}-{}", file!(), line!())
                });

                stream.write(&size.to_be_bytes()).map_err(|e| {
                    context!(
                        e,
                        "error writing payload size: {} to address: {:?}",
                        size,
                        addr
                    )
                })?;
                stream
                    .write(&json.as_bytes())
                    .map_err(|e| context!(e, "error writing JSON data to address: {:?}", addr))?;

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
    fn exit_code(&mut self) -> AppResult<Option<i32>> {
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
            Err(e) => {
                return Err(context!(
                    e,
                    "error waiting for child for path:{:?}",
                    self.child
                ))
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use regex::Regex;
    use std::borrow::Cow;
    use std::io::{Error, ErrorKind, Result};
    use std::str::FromStr;

    use crate::configuration::vars::Vars;

    #[derive(Debug, Deserialize)]
    struct JSONStream {
        pub args: Vec<String>,
        pub vars: Vars<String, String>,
    }

    // utility fn to receive JSON from a stream
    fn get_json_from_stream<T: std::io::Read>(socket: &mut T) -> Result<JSONStream> {
        // try to read size first
        let mut size_buffer = [0; std::mem::size_of::<u16>()];
        let bytes_read = socket.read(&mut size_buffer)?;
        //dbg!(bytes_read);
        if bytes_read == 0 {
            return Err(Error::new(ErrorKind::Interrupted, "socket closed"));
        }

        let json_size = u16::from_be_bytes(size_buffer);

        // read JSON raw data
        let mut json_buffer = vec![0; json_size as usize];
        socket.read_exact(&mut json_buffer).unwrap();

        // get JSON
        let s = std::str::from_utf8(&json_buffer).unwrap();

        let json: JSONStream = serde_json::from_str(&s).unwrap();
        Ok(json)
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn callback_script() {
        let yaml = r#"
            script: "tests/unittest/callback_script.py"
            args: ['one', 'two', 'three']
        "#;

        let cb: Callback = Callback::from_str(yaml).expect("unable to read YAML");
        let script = PathBuf::from("tests/unittest/callback_script.py");
        assert!(matches!(&cb.callback, CallbackType::Script(Some(x)) if x == &script));
        assert_eq!(cb.args.as_ref().unwrap().len(), 3);

        // create dummy variables
        let re = Regex::new(r"^([a-z\s]+) (\w+) (\w+) (?P<LASTNAME>\w+)").unwrap();
        let text = "my name is john fitzgerald kennedy, president of the USA";

        let mut vars = Vars::<Cow<str>, &str>::default();
        vars.insert_captures(&re, text);

        // call script
        let mut handle = CallbackHandle::default();
        let data = cb
            .call(None, &GlobalVars::default(), &vars, &mut handle)
            .unwrap();
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
                    let json = get_json_from_stream(&mut socket)
                        .expect("unable to get JSON data from stream");

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
        let re = Regex::new(r"^([a-z\s]+) (\w+) (\w+) (?P<LASTNAME>\w+)").unwrap();
        let text = "my name is john fitzgerald kennedy, president of the USA";

        let mut vars = Vars::<Cow<str>, &str>::default();
        vars.insert_captures(&re, text);

        // some work here
        let mut handle = CallbackHandle::default();
        let data = cb
            .call(None, &GlobalVars::default(), &vars, &mut handle)
            .unwrap();
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
                    let json = get_json_from_stream(&mut socket)
                        .expect("unable to get JSON data from stream");

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
        let re = Regex::new(r"^([a-z\s]+) (\w+) (\w+) (?P<LASTNAME>\w+)").unwrap();
        let text = "my name is john fitzgerald kennedy, president of the USA";

        let mut vars = Vars::<Cow<str>, &str>::default();
        vars.insert_captures(&re, text);

        // some work here
        let mut handle = CallbackHandle::default();
        let data = cb
            .call(None, &GlobalVars::default(), &mut vars, &mut handle)
            .unwrap();
        assert!(data.is_none());

        //cb.call(None, &vars).unwrap();

        let _res = child.join();
    }
}
