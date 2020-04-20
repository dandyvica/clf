use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Seek, SeekFrom};

use flate2::read::GzDecoder;

/// A simple trait for using read_line() on ClfBufReader
pub trait ClfBufRead {
    fn read_line(&mut self, buf: &mut String) -> Result<usize, std::io::Error>;
}

/// A custom BufReader, as GzDecoder<File> doesn't implement the Seek trait.
pub struct ClfBufReader<R> {
    reader: BufReader<R>,
    current_line: u64,
    current_byte: u64,
}

impl<T> ClfBufReader<T> {
    pub fn new(stream: T) -> ClfBufReader<T>
    where
        T: Read,
    {
        ClfBufReader {
            reader: BufReader::new(stream),
            current_line: 0,
            current_byte: 0,
        }
    }
}

/// A reimplementation of a dedicated BufReader, because GzDecoder<File> doesn't
/// implement the Seek trait.
impl ClfBufRead for ClfBufReader<File> {
    /// A mere call to BufReader<File>::read_line()
    fn read_line(&mut self, buf: &mut String) -> Result<usize, std::io::Error> {
        self.reader.read_line(buf)
    }
}

/// As GzDecoder<File> doesn't implement Seek, we need to provide a custome pseudo seek() function.
/// A a seek reset is not provided, the SeekFrom(Start) will only be possible just after opening the
/// gzipped stream, before any read.
impl ClfBufRead for ClfBufReader<GzDecoder<File>> {
    /// A mere call to BufReader<File>::read_line(), but in addition, sets some internal
    /// values to keep offset internally.
    fn read_line(&mut self, buf: &mut String) -> Result<usize, std::io::Error> {
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
    use std::io::{Seek, SeekFrom};

    use flate2::read::GzDecoder;

    use crate::bufreader::{tests, ClfBufRead, ClfBufReader};
    use crate::setup::{create_ascii, create_gzip};

    #[test]
    fn test_bufreader_file() {
        let ascii_file = create_ascii("az_ascii.txt");
        let mut bufreader = ClfBufReader::new(File::open(ascii_file).unwrap());
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
        // prepare test
        let gzip_file_name = tests::create_gzip("az_gzip.txt.gz");
        let gzip_file = File::open(&gzip_file_name).unwrap();
        let decoder = GzDecoder::new(gzip_file);
        let mut bufreader = ClfBufReader::new(decoder);
        let mut s = String::new();

        // test seek & read_line()
        let _ = bufreader.seek(SeekFrom::Start(10));
        let _ = bufreader.read_line(&mut s);
        assert_eq!(s, "KLMNOPQRSTUVWXYZ\n");
        s.clear();

        let _ = bufreader.read_line(&mut s);
        assert_eq!(s, "BCDEFGHIJKLMNOPQRSTUVWXYZA\n");
        s.clear();
        assert_eq!(bufreader.seek(SeekFrom::Current(0)).unwrap(), 44);

        let _ = bufreader.read_line(&mut s);
        assert_eq!(s, "CDEFGHIJKLMNOPQRSTUVWXYZAB\n");
        s.clear();
        assert_eq!(bufreader.seek(SeekFrom::Current(0)).unwrap(), 71);
    }

    #[test]
    #[should_panic]
    fn test_bufreader_gzip_fromend() {
        // prepare test
        let gzip_file_name = tests::create_gzip("az_gzip.txt.gz");
        let gzip_file = File::open(&gzip_file_name).unwrap();
        let decoder = GzDecoder::new(gzip_file);
        let mut bufreader = ClfBufReader::new(decoder);        

        let _ = bufreader.seek(SeekFrom::End(0));
    }

    #[test]
    #[should_panic]
    fn test_bufreader_gzip_current() {
        // prepare test
        let gzip_file_name = tests::create_gzip("az_gzip.txt.gz");
        let gzip_file = File::open(&gzip_file_name).unwrap();
        let decoder = GzDecoder::new(gzip_file);
        let mut bufreader = ClfBufReader::new(decoder);

        let _ = bufreader.seek(SeekFrom::Current(10));
    }

    #[test]
    fn test_bufreader_gzip_eof() {
        // prepare test
        let gzip_file_name = tests::create_gzip("az_gzip.txt.gz");
        let gzip_file = File::open(&gzip_file_name).unwrap();
        let decoder = GzDecoder::new(gzip_file);
        let mut bufreader = ClfBufReader::new(decoder);        

        assert_eq!(
            bufreader.seek(SeekFrom::Start(1000)).unwrap_err().kind(),
            std::io::ErrorKind::UnexpectedEof
        );
    }
}
