use serde::Deserialize;
use std::io::{BufReader, Cursor, Error, ErrorKind, Read};

use flate2::bufread::GzEncoder;
use flate2::Compression;

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

// compress data with a compressor
pub fn gzip_data(data: &str) -> Vec<u8> {
    let cursor = Cursor::new(data);

    let bufreader = BufReader::new(cursor);
    let mut gz = GzEncoder::new(bufreader, Compression::fast());
    let mut buffer = Vec::new();
    gz.read_to_end(&mut buffer).expect("unable to gzip");

    buffer
}
