//! Manage different types of compression for a logfile
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use xz2::read::XzDecoder;

use crate::{
    logfile::compression::CompressionScheme,
    misc::error::{AppCustomErrorKind, AppError},
};

pub enum LogReader<R: Read> {
    Gzip(BufReader<GzDecoder<R>>),
    Bzip2(BufReader<BzDecoder<R>>),
    Xz(BufReader<XzDecoder<R>>),
    Uncompressed(BufReader<R>),
}

impl Read for LogReader<File> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            LogReader::Gzip(f) => f.read(buf),
            LogReader::Bzip2(f) => f.read(buf),
            LogReader::Xz(f) => f.read(buf),
            LogReader::Uncompressed(f) => f.read(buf),
        }
    }
}

impl LogReader<File> {
    pub fn read_until(&mut self, byte: u8, buf: &mut Vec<u8>) -> std::io::Result<usize> {
        match self {
            LogReader::Gzip(f) => f.read_until(byte, buf),
            LogReader::Bzip2(f) => f.read_until(byte, buf),
            LogReader::Xz(f) => f.read_until(byte, buf),
            LogReader::Uncompressed(f) => f.read_until(byte, buf),
        }
    }

    /// Creates a new reader depending on the compression. This reader can be used as a regular `BufReader` struct.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, AppError> {
        // open target file
        let file = File::open(path.as_ref())?;

        let path = PathBuf::from(path.as_ref());
        let extension = path.extension().map(|x| x.to_string_lossy().to_string());
        let compression = CompressionScheme::from(extension.as_deref());

        // create a specific reader for each compression scheme
        match compression {
            CompressionScheme::Gzip => {
                let decoder = GzDecoder::new(file);
                let reader = BufReader::new(decoder);
                Ok(LogReader::Gzip(reader))
            }
            CompressionScheme::Bzip2 => {
                let decoder = BzDecoder::new(file);
                let reader = BufReader::new(decoder);
                Ok(LogReader::Bzip2(reader))
            }
            CompressionScheme::Xz => {
                let decoder = XzDecoder::new(file);
                let reader = BufReader::new(decoder);
                Ok(LogReader::Xz(reader))
            }
            CompressionScheme::Uncompressed => {
                let reader = BufReader::new(file);
                Ok(LogReader::Uncompressed(reader))
            }
        }
    }

    pub fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
        match self {
            LogReader::Uncompressed(f) => f.seek(SeekFrom::Start(offset)).map_err(AppError::Io),
            f @ _ => set_offset(f, offset),
        }
    }
}

// This method is common to all compression ad-hoc seek method.
fn set_offset<R>(mut reader: R, offset: u64) -> Result<u64, AppError>
where
    R: Read,
{
    // if 0, nothing to do
    if offset == 0 {
        return Ok(0);
    }

    let pos = match reader.by_ref().bytes().nth((offset - 1) as usize) {
        None => {
            return Err(AppError::new(
                AppCustomErrorKind::SeekPosBeyondEof,
                &format!("tried to set offset beyond EOF, at offset: {}", offset),
            ))
        }
        Some(x) => x,
    };
    Ok(pos.unwrap() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_compressed_reader() -> LogReader<File> {
        let reader = LogReader::<File>::from_path("tests/logfiles/clftest.txt.gz")
            .expect("unable to create compressed reader");
        reader
    }

    #[test]
    fn reader() {
        let mut reader = get_compressed_reader();
        let mut buffer = [0; 1];

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
}
