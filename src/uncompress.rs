//! An executable to read compressed files using compression methods defined in the crate
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use clap::{App, Arg};

use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use xz2::read::XzDecoder;

use clf::logfile::compression::CompressionScheme;

fn main() {
    let matches = App::new("Uncompress gzip, bzip2, xz files")
    .version("0.1")
    .author("Alain Viguier dandyvica@gmail.com")
    .about(r#"An executable to read compressed files using compression methods defined in the crate"#)
    .arg(
        Arg::with_name("file")
            .long_help("Mandatory argument. The name and path of the YAML configuration file, containing logfiles to search for and patterns to match.")
            .short("f")
            .long("file")
            .required(true)
            .help("Name of compresse file.")
            .takes_value(true),
    )            .get_matches();

    // get file name
    let path = PathBuf::from(matches.value_of("file").unwrap());
    let file = File::open(&path).expect(&format!("unable to open file {:?}", &path));

    // get file extension
    let extension = path.extension().map(|x| x.to_string_lossy().to_string());
    let scheme = CompressionScheme::from(extension.as_deref());

    // create extension scheme and reader
    match scheme {
        CompressionScheme::Gzip => {
            let decoder = GzDecoder::new(file);
            let reader = BufReader::new(decoder);
            read_file(reader);
        }
        CompressionScheme::Bzip2 => {
            let decoder = BzDecoder::new(file);
            let reader = BufReader::new(decoder);
            read_file(reader);
        }
        CompressionScheme::Xz => {
            let decoder = XzDecoder::new(file);
            let reader = BufReader::new(decoder);
            read_file(reader);
        }
        CompressionScheme::Uncompressed => {
            let reader = BufReader::new(file);
            read_file(reader);
        }
    };
}

fn read_file<R: BufRead>(mut reader: R) {
    // our read buffer
    let mut buffer = Vec::with_capacity(1024);

    loop {
        let ret = reader.read_until(b'\n', &mut buffer);
        if let Ok(bytes_read) = ret {
            if bytes_read == 0 {
                break;
            }
        }

        let line = String::from_utf8_lossy(&buffer);
        print!("{}", line);

        buffer.clear();
    }
}
