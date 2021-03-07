#![allow(warnings, unused)]
use std::env;
use std::fs::File;
use std::io::prelude::*;

mod testcase;
use testcase::JSONStream;

fn main() -> std::io::Result<()> {
    #[cfg(target_family = "unix")]
    {
        let args: Vec<String> = env::args().collect();
        let mut file: Option<File> = None;

        // one argument is mandatory: address and port
        let addr = &args[1];
        let _ = std::fs::remove_file(&addr);

        if args.len() == 3 {
            file = Some(File::create(&args[2]).unwrap());
        }

        let listener = std::os::unix::net::UnixListener::bind(addr).unwrap();
        println!("waiting on address: {}", addr);
        match listener.accept() {
            Ok((mut socket, _addr)) => {
                // set short timeout
                // socket
                //     .set_read_timeout(Some(std::time::Duration::new(3, 0)))
                //     .expect("Couldn't set read timeout");

                // loop to receive data
                loop {
                    let json = JSONStream::get_json_from_stream(&mut socket);
                    if json.is_err() {
                        break;
                    }

                    let j = json.unwrap();
                    let serialized = serde_json::to_string(&j).unwrap();

                    if file.is_some() {
                        let _ = write!(file.as_ref().unwrap(), "{}\n", serialized);
                    } else {
                        println!("{}", serialized)
                    }
                }
            }
            Err(e) => panic!("couldn't get client: {:?}", e),
        }
    }
    Ok(())
}
