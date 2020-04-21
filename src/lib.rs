#[macro_use]
pub mod error;
pub mod logfile;
//pub mod lookup;
//pub mod serdeser;
pub mod bufreader;
pub mod search;
//pub mod file_iter;
pub mod pattern;

mod setup {
    use std::fs::File;
    use std::io::Write;

    use flate2::{Compression, GzBuilder};

    // create a temporary file to test our modules
    pub fn create_ascii(name: &str) -> std::path::PathBuf {
        let az = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(2);

        let mut ascii_file = std::env::temp_dir();
        ascii_file.push(name);

        let mut f = File::create(&ascii_file)
            .expect("unable to create temporary ASCII file for unit tests");
        for i in 0..26 {
            f.write(az[i..i + 26].as_bytes()).unwrap();
            if cfg!(unix) {
                f.write("\n".as_bytes()).unwrap();
            } else if cfg!(windows) {
                f.write("\r\n".as_bytes()).unwrap();
            } else {
                unimplemented!("create_ascii(): not yet implemented");
            }
        }

        ascii_file
    }

    // create a temporary gzipped file to test our BufReader
    pub fn create_gzip(name: &str) -> std::path::PathBuf {
        let az = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(2);

        let mut gzip_file = std::env::temp_dir();
        gzip_file.push(name);
        //gzip_file.set_extension("gz");

        let f =
            File::create(&gzip_file).expect("unable to create temporary gzip file for unit tests");
        let mut gz = GzBuilder::new()
            //.filename("foo.txt")
            .write(f, Compression::default());

        for i in 0..26 {
            gz.write(az[i..i + 26].as_bytes()).unwrap();
            if cfg!(unix) {
                gz.write("\n".as_bytes()).unwrap();
            } else if cfg!(windows) {
                gz.write("\r\n".as_bytes()).unwrap();
            } else {
                unimplemented!("create_ascii(): not yet implemented");
            }
        }
        gz.finish().unwrap();

        gzip_file
    }
}
