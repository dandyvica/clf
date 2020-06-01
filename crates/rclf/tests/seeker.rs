use std::fs::File;
use std::io::{BufReader, Read};

use flate2::read::GzDecoder;

use rclf::logfile::Seeker;

fn get_compressed_reader() -> BufReader<GzDecoder<File>> {
    let file = File::open("tests/assets/clftest.gz").expect("unable to open compressed test file");
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);

    reader
}

#[test]
fn test_seeker_uncompressed_file() {
    let file = File::open("tests/assets/clftest.txt").expect("unable to open test file");
    let mut reader = BufReader::new(file);
    let mut buffer = [0; 1];

    let mut a = reader.set_offset(1);
    assert!(a.is_ok());
    reader.read_exact(&mut buffer).unwrap();
    assert_eq!(buffer[0], 'B' as u8);

    a = reader.set_offset(26);
    assert!(a.is_ok());
    reader.read_exact(&mut buffer).unwrap();
    assert_eq!(buffer[0], '\n' as u8);

    a = reader.set_offset(0);
    assert!(a.is_ok());
    reader.read_exact(&mut buffer).unwrap();
    assert_eq!(buffer[0], 'A' as u8);
}

#[test]
fn test_seeker_compressed_file() {
    let mut buffer = [0; 1];

    let mut reader = get_compressed_reader();
    let mut offset = reader.set_offset(1);
    assert!(offset.is_ok());
    reader.read_exact(&mut buffer).unwrap();
    assert_eq!(buffer[0], 'B' as u8);

    reader = get_compressed_reader();
    offset = reader.set_offset(0);
    assert!(offset.is_ok());
    reader.read_exact(&mut buffer).unwrap();
    assert_eq!(buffer[0], 'A' as u8);

    reader = get_compressed_reader();
    offset = reader.set_offset(26);
    assert!(offset.is_ok());
    reader.read_exact(&mut buffer).unwrap();
    assert_eq!(buffer[0], '\n' as u8);

    reader = get_compressed_reader();
    offset = reader.set_offset(25);
    assert!(offset.is_ok());
    reader.read_exact(&mut buffer).unwrap();
    assert_eq!(buffer[0], 'Z' as u8);

    reader = get_compressed_reader();
    offset = reader.set_offset(10000);
    assert!(offset.is_err());
}
