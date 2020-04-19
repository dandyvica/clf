use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Seek, SeekFrom};

use flate2::read::GzDecoder;

/// A custom BufReader, as GzDecoder<File> doesn't implement the Seel trait.
pub struct ClfBufReader<R> {
    reader: BufReader<R>,
    current_line: u64,
    current_byte: u64,
}

/// A reimplementation of a dedicated BufReader, because GzDecoder<File> doesn't
/// implement the Seek trait.
impl ClfBufReader<File> {
    /// Creates a new ClfBufReader<File>. Nothing special here.
    pub fn new(file: File) -> Self {
        ClfBufReader {
            reader: BufReader::new(file),
            current_line: 0,
            current_byte: 0,
        }
    }

    /// A mere call to BufReader<File>::read_line()
    pub fn read_line(&mut self, buf: &mut String) -> Result<usize, std::io::Error> {
        self.reader.read_line(buf)
    }
}

/// As GzDecoder<File> doesn't implement Seek, we need to provide a custome pseudo seek() function.
/// A a seek reset is not provided, the SeekFrom(Start) will only be possible just after opening the
/// gzipped stream, before any read.
impl ClfBufReader<GzDecoder<File>> {
    /// Creates a new ClfBufReader<GzDecoder<File>>. Nothing special here.
    pub fn new(file: File) -> Self {
        ClfBufReader {
            reader: BufReader::new(GzDecoder::new(file)),
            current_line: 0,
            current_byte: 0,
        }
    }

    /// A mere call to BufReader<File>::read_line(), but in addition, sets some internal
    /// values to keep offset internally.
    pub fn read_line(&mut self, buf: &mut String) -> Result<usize, std::io::Error> {
        match self.reader.read_line(buf) {
            Err(e) => Err(e),
            Ok(nb_read) => {
                if nb_read != 0 {
                    self.current_line += 1;
                    self.current_byte += nb_read as u64;
                }
                Ok(nb_read)
            }
        }
    }
}

/// A simple blanket Seek trait implementation.
impl Seek for ClfBufReader<File> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error> {
        self.reader.seek(pos)
    }
}

/// A simple blanket Seek implementation for GzDecoder<File>. Nothat SeekFrom::End is not
/// allowed because impossible.
impl Seek for ClfBufReader<GzDecoder<File>> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, std::io::Error> {
        match pos {
            SeekFrom::Start(offset) => {
                if offset == 0 {
                    Ok(0)
                } else {
                    match self.reader.by_ref().bytes().nth((offset - 1) as usize) {
                        None => Err(Error::new(ErrorKind::UnexpectedEof, "offset is beyond EOF")),
                        Some(x) => Ok(offset),
                    }
                }
            }
            SeekFrom::End(_) => {
                unimplemented!("SeekFrom::End not implemented for ClfBufReader<GzDecoder<File>>")
            }
            SeekFrom::Current(offset) => {
                if offset != 0 {
                    unimplemented!(
                        "SeekFrom::Current not implemented for ClfBufReader<GzDecoder<File>>"
                    );
                }
                Ok(self.current_byte)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{Seek, SeekFrom, Write};

    use flate2::{read::GzDecoder, Compression, GzBuilder};

    use crate::reader::{tests, ClfBufReader};

    // create a temporary file to test our BufReader
    fn create_ascii() -> std::path::PathBuf {
        let az = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(2);

        let mut ascii_file = std::env::temp_dir();
        ascii_file.push("az_ascii.txt");

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
    fn create_gzip() -> std::path::PathBuf {
        let az = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(2);

        let mut gzip_file = std::env::temp_dir();
        gzip_file.push("az.gz");

        let mut f =
            File::create(&gzip_file).expect("unable to create temporary gzip file for unit tests");
        let mut gz = GzBuilder::new()
            .filename("az_gzip.txt")
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

        gzip_file
    }

    #[test]
    fn test_bufreader_file() {
        let mut bufreader = ClfBufReader::<File>::new(File::open(tests::create_ascii()).unwrap());
        let mut s = String::new();

        let _ = bufreader.read_line(&mut s);
        assert_eq!(s, "ABCDEFGHIJKLMNOPQRSTUVWXYZ\n");
        s.clear();
        assert_eq!(bufreader.seek(SeekFrom::Current(0)).unwrap(), 27);

        let _ = bufreader.read_line(&mut s);
        assert_eq!(s, "BCDEFGHIJKLMNOPQRSTUVWXYZA\n");
        s.clear();
        assert_eq!(bufreader.seek(SeekFrom::Current(0)).unwrap(), 54);

        let _ = bufreader.seek(SeekFrom::Start(10));
        let _ = bufreader.read_line(&mut s);
        assert_eq!(s, "KLMNOPQRSTUVWXYZ\n");
        s.clear();
    }

    #[test]
    fn test_bufreader_gzip() {
        let gzip_file = tests::create_gzip();

        let mut bufreader =
            ClfBufReader::<GzDecoder<File>>::new(File::open(&gzip_file).unwrap());
        let mut s = String::new();

        let _ = bufreader.seek(SeekFrom::Start(10));
        let _ = bufreader.read_line(&mut s);
        assert_eq!(s, "KLMNOPQRSTUVWXYZ\n");
        s.clear();

        let mut bufreader2 =
            ClfBufReader::<GzDecoder<File>>::new(File::open(&gzip_file).unwrap());

        let _ = bufreader2.read_line(&mut s);
        assert_eq!(s, "ABCDEFGHIJKLMNOPQRSTUVWXYZ\n");
        s.clear();
        assert_eq!(bufreader2.seek(SeekFrom::Current(0)).unwrap(), 27);

        let _ = bufreader2.read_line(&mut s);
        assert_eq!(s, "BCDEFGHIJKLMNOPQRSTUVWXYZA\n");
        s.clear();
        assert_eq!(bufreader2.seek(SeekFrom::Current(0)).unwrap(), 54);
    }

    #[test]
    #[should_panic]
    fn test_bufreader_gzip_fromend() {
        let mut bufreader =
            ClfBufReader::<GzDecoder<File>>::new(File::open(tests::create_gzip()).unwrap());

        let _ = bufreader.seek(SeekFrom::End(0));
    }

    #[test]
    #[should_panic]
    fn test_bufreader_gzip_current() {
        let mut bufreader =
            ClfBufReader::<GzDecoder<File>>::new(File::open(tests::create_gzip()).unwrap());

        let _ = bufreader.seek(SeekFrom::Current(10));
    }

    #[test]
    fn test_bufreader_gzip_eof() {
        let mut bufreader =
            ClfBufReader::<GzDecoder<File>>::new(File::open(tests::create_gzip()).unwrap());

        assert_eq!(
            bufreader.seek(SeekFrom::Start(1000)).unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }
}
