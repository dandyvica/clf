//! Useful wrapper on the `Command` Rust standard library structure.
use std::cell::RefCell;
use std::io::Write;
use std::net::TcpStream;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::Instant;

use log::{debug, info};
use serde::Deserialize;
use serde_json::json;

/// Returns the number of seconds for a standard timeout when not defined in the YAML file.
/// Neede by `serde`.
const fn default_timeout() -> u64 {
    2 * 3600
}

use crate::variables::Variables;
use misc::error::AppError;

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
    pub(in crate) id: CallbackType,

    /// Option arguments of the previous.
    pub(in crate) args: Option<Vec<String>>,

    /// A timeout in seconds to for wait command completion.
    #[serde(default = "self::default_timeout")]
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
            &self.id,
            self.args,
            env_path,
            vars,
            std::env::current_dir()
                .map_err(|e| format!("unable to fetch current directory, error={}", e))
        );        


        // the callback is called depending of its type
        match &self.id {
            CallbackType::Script(path) => {
                // build Command struct before execution.
                debug_assert!(path.is_some());

                // test whether a Command struct is already created
                if handle.cmd.is_none() {
                    let cmd = Command::new(path.as_ref().unwrap());
                    handle.cmd = Some(cmd);
                    debug!("creating Command for: {:?}", path.as_ref().unwrap());
                }

                let cmd = handle.cmd.as_mut().unwrap();

                // runtime variables are always there.
                cmd.envs(&vars.runtime_vars);

                // user variables, maybe
                if let Some(uservars) = vars.user_vars() {
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
                let json = json!({
                    "args": &self.args,
                    "vars": vars
                })
                .to_string();

                debug!(
                    "sending JSON data to TCP socket: {}",
                    address.as_ref().unwrap()
                );
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
                let json = json!({
                    "args": &self.args,
                    "vars": vars
                })
                .to_string();

                debug!(
                    "sending JSON data to UNIX socket: {:?}",
                    address.as_ref().unwrap()
                );
                stream.write_all(&json.as_bytes())?;

                Ok(None)
            }
        }
    }
}

/// Return structure from a call to a script. Gathers all relevant data, instead of a mere tuple.
#[derive(Debug, Default)]
pub struct ChildData {
    pub child: Option<RefCell<Child>>,
    pub path: PathBuf,
    pub timeout: u64,
    pub start_time: Option<Instant>,
}

impl ChildData {
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
            Err(e) => return Err(misc::error::AppError::Io(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;
    use std::io::prelude::*;

    #[derive(Deserialize)]
    struct JSON {
        args: Vec<String>,
        vars: Variables,
    }

    // helper fn to create a dummy Variables struct
    fn dummy_vars() -> Variables {
        // create dummy variables
        let re = Regex::new(r"^([a-z\s]+) (\w+) (\w+) (?P<LASTNAME>\w+)").unwrap();
        let text = "my name is john fitzgerald kennedy, president of the USA";

        let mut vars = Variables::new();
        vars.insert_captures(&re, text);

        vars
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn callback_script() {
        let yaml = r#"
            script: "tests/check_ut.py"
            args: ['one', 'two', 'three']
        "#;

        let cb: Callback = serde_yaml::from_str(yaml).expect("unable to read YAML");
        let script = PathBuf::from("tests/check_ut.py");
        assert!(matches!(&cb.id, CallbackType::Script(Some(x)) if x == &script));
        assert_eq!(cb.args.as_ref().unwrap().len(), 3);

        // create dummy variables
        let vars = super::tests::dummy_vars();

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

        let cb: Callback = serde_yaml::from_str(yaml).expect("unable to read YAML");
        let addr = "127.0.0.1:8900".to_string();
        assert!(matches!(&cb.id, CallbackType::Tcp(Some(x)) if x == &addr));

        // create a very simple TCP server: wait for data and test them
        let child = std::thread::spawn(move || {
            // create a listener
            let listener = std::net::TcpListener::bind(&addr).unwrap();
            match listener.accept() {
                Ok((mut socket, _addr)) => {
                    //println!("new client: {:?}", addr);

                    let mut buffer = vec![0; 1024];
                    socket.read(&mut buffer).unwrap();

                    let s = std::str::from_utf8(&buffer)
                        .unwrap()
                        .trim_end_matches(char::from(0));
                    //println!("data={}", s);
                    //println!("data={:?}", buffer);

                    let json: JSON = serde_json::from_str(&s).unwrap();

                    assert_eq!(json.args, vec!["one", "two", "three"]);

                    assert_eq!(
                        json.vars.get_runtime_var("CLF_CAPTURE1").unwrap(),
                        "my name is"
                    );
                    assert_eq!(json.vars.get_runtime_var("CLF_CAPTURE2").unwrap(), "john");
                    assert_eq!(
                        json.vars.get_runtime_var("CLF_CAPTURE3").unwrap(),
                        "fitzgerald"
                    );
                    assert_eq!(
                        json.vars.get_runtime_var("CLF_LASTNAME").unwrap(),
                        "kennedy"
                    );
                }
                Err(e) => panic!("couldn't get client: {:?}", e),
            }
        });

        // wait a little
        let ten_millis = std::time::Duration::from_millis(10);
        std::thread::sleep(ten_millis);

        // create dummy variables
        let vars = super::tests::dummy_vars();

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

        let cb: Callback = serde_yaml::from_str(yaml).expect("unable to read YAML");
        let addr = PathBuf::from("/tmp/callback.sock");

        let _ = std::fs::remove_file(&addr);

        assert!(matches!(&cb.id, CallbackType::Domain(Some(x)) if x == &addr));

        // create a very simple UNIX socket server: wait for data and test them
        let child = std::thread::spawn(move || {
            // create a listener
            let listener = std::os::unix::net::UnixListener::bind(addr).unwrap();
            match listener.accept() {
                Ok((mut socket, _addr)) => {
                    let mut buffer = vec![0; 1024];
                    socket.read(&mut buffer).unwrap();

                    let s = std::str::from_utf8(&buffer)
                        .unwrap()
                        .trim_end_matches(char::from(0));
                    //println!("data={:?}", buffer);
                    //println!("data={}", s);
                    let json: JSON = serde_json::from_str(&s).unwrap();

                    assert_eq!(json.args, vec!["one", "two", "three"]);

                    assert_eq!(
                        json.vars.get_runtime_var("CLF_CAPTURE1").unwrap(),
                        "my name is"
                    );
                    assert_eq!(json.vars.get_runtime_var("CLF_CAPTURE2").unwrap(), "john");
                    assert_eq!(
                        json.vars.get_runtime_var("CLF_CAPTURE3").unwrap(),
                        "fitzgerald"
                    );
                    assert_eq!(
                        json.vars.get_runtime_var("CLF_LASTNAME").unwrap(),
                        "kennedy"
                    );
                }
                Err(e) => panic!("couldn't get client: {:?}", e),
            }
        });

        // wait a little
        let ten_millis = std::time::Duration::from_millis(10);
        std::thread::sleep(ten_millis);

        // create dummy variables
        let mut vars = super::tests::dummy_vars();

        // some work here
        let mut handle = CallbackHandle::default();
        let data = cb.call(None, &mut vars, &mut handle).unwrap();
        assert!(data.is_none());

        //cb.call(None, &vars).unwrap();

        let _res = child.join();
    }
}
