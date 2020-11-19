//! As compression decoders don't implement the `Seek`trait, we need to define a sibling one with another name
//! due to error E0119: "There are conflicting trait implementations for the same type."
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use bzip2::read::BzDecoder;
use flate2::read::GzDecoder;
use xz2::read::XzDecoder;

use crate::misc::error::{AppCustomErrorKind, AppError};

pub trait Seeker {
    /// Simulates the `seek`method for all used `BufReader<R>`.
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError>;
}

impl Seeker for BufReader<File> {
    #[inline(always)]
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
        self.seek(SeekFrom::Start(offset)).map_err(AppError::Io)
    }
}

/// Implementing for `R: Read` helps testing wuth `Cursor` type.
impl<R> Seeker for BufReader<GzDecoder<R>>
where
    R: Read,
{
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
        _set_offset(self, offset)
    }
}

impl<R> Seeker for BufReader<BzDecoder<R>>
where
    R: Read,
{
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
        _set_offset(self, offset)
    }
}

impl<R> Seeker for BufReader<XzDecoder<R>>
where
    R: Read,
{
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
        _set_offset(self, offset)
    }
}

// This method is common to all compression ad-hoc seek method.
fn _set_offset<R>(mut reader: R, offset: u64) -> Result<u64, AppError>
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

    //     fn get_compressed_reader() -> BufReader<GzDecoder<Cursor<Vec<u8>>>> {
    //         let data = r#"
    // ABCDEFGHIJKLMNOPQRSTUVWXYZ
    // BCDEFGHIJKLMNOPQRSTUVWXYZA
    // CDEFGHIJKLMNOPQRSTUVWXYZAB
    // DEFGHIJKLMNOPQRSTUVWXYZABC
    // EFGHIJKLMNOPQRSTUVWXYZABCD
    // FGHIJKLMNOPQRSTUVWXYZABCDE
    // GHIJKLMNOPQRSTUVWXYZABCDEF
    // HIJKLMNOPQRSTUVWXYZABCDEFG
    // IJKLMNOPQRSTUVWXYZABCDEFGH
    // JKLMNOPQRSTUVWXYZABCDEFGHI
    // KLMNOPQRSTUVWXYZABCDEFGHIJ
    // LMNOPQRSTUVWXYZABCDEFGHIJK
    // MNOPQRSTUVWXYZABCDEFGHIJKL
    // NOPQRSTUVWXYZABCDEFGHIJKLM
    // OPQRSTUVWXYZABCDEFGHIJKLMN
    // PQRSTUVWXYZABCDEFGHIJKLMNO
    // QRSTUVWXYZABCDEFGHIJKLMNOP
    // RSTUVWXYZABCDEFGHIJKLMNOPQ
    // STUVWXYZABCDEFGHIJKLMNOPQR
    // TUVWXYZABCDEFGHIJKLMNOPQRS
    // UVWXYZABCDEFGHIJKLMNOPQRST
    // VWXYZABCDEFGHIJKLMNOPQRSTU
    // WXYZABCDEFGHIJKLMNOPQRSTUV
    // XYZABCDEFGHIJKLMNOPQRSTUVW
    // YZABCDEFGHIJKLMNOPQRSTUVWX
    // ZABCDEFGHIJKLMNOPQRSTUVWXY
    // "#;

    //         // gzip our data
    //         let gzip_data = gzip_data(&data);

    //         let cursor = std::io::Cursor::new(&gzip_data);
    //         let decoder = GzDecoder::new(cursor);
    //         let reader = BufReader::new(decoder);

    //         reader
    //     }

    fn get_compressed_reader() -> BufReader<GzDecoder<File>> {
        let file = File::open("tests/logfiles/clftest.txt.gz")
            .expect("unable to open compressed test file");
        let decoder = GzDecoder::new(file);
        let reader = BufReader::new(decoder);

        reader
    }

    #[test]
    fn set_offset() {
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
}
