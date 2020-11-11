use serde::Deserialize;
use std::io::{Error, ErrorKind};

use super::data::JSONStream;

// Load a JSON string as a structure
pub fn load_json<'a, T: Deserialize<'a>>(json: &'a str) -> T {
    serde_json::from_str(json).expect("unable to load JSON")
}

// utility fn to receive JSON from a stream
pub fn get_json_from_stream<T: std::io::Read>(
    socket: &mut T,
) -> Result<JSONStream, std::io::Error> {
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
