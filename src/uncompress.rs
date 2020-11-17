//! An executable to read compressed files using compression methods defined in the crate
use std::path::PathBuf;

use clap::{App, Arg};

use clf::logfile::logreader::LogReader;

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

    // our read buffer
    let mut buffer = Vec::with_capacity(1024);

    // create extension scheme and reader
    let mut reader = LogReader::from_path(&path).unwrap();

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
