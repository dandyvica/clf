//! An executable to read compressed files using compression methods defined in the crate
use std::io::BufRead;
use std::path::PathBuf;

use clap::{App, Arg};

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

    // get file extension
    let extension = path.extension().map(|x| x.to_string_lossy().to_string());

    // create extension scheme and reader
    let scheme = CompressionScheme::from(extension.as_deref());
    let mut reader = scheme.reader(&path).unwrap();

    // just print out every line
    for line in reader.lines() {
        println!("{}", line.unwrap());
    }

}
