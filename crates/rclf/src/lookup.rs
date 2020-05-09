//! Traits dedicated to search patterns in a `LogFile`.
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::SystemTime;

use flate2::read::{GzDecoder, ZlibDecoder};
use log::{debug, info};

//use crate::bufreader::{ClfBufRead, ClfBufReader};
use crate::config::Tag;
use crate::error::{AppCustomErrorKind, AppError};
use crate::logfile::{LogFile, RunData};
use crate::pattern::PatternSet;

/// Trait which provides a seek function, and is implemented for all
/// `BufReader<T>` types used in `Lookup` trait.
pub trait Seeker {
    /// Simulates the `seek`method for all used `BufReader<R>`.
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError>;
}

impl Seeker for BufReader<File> {
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
        self.seek(SeekFrom::Start(offset))
            .map_err(|e| AppError::Io(e))
    }
}

impl Seeker for BufReader<GzDecoder<File>> {
    fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
        // if 0, nothing to do
        if offset == 0 {
            return Ok(0);
        }

        let pos = match self.by_ref().bytes().nth((offset - 1) as usize) {
            None => {
                return Err(AppError::App {
                    err: AppCustomErrorKind::SeekPosBeyondEof,
                    msg: format!("tried to set offset beyond EOF, at offset: {}", offset),
                })
            }
            Some(x) => x,
        };
        Ok(pos.unwrap() as u64)
    }
}

// impl Seeker for BufReader<ZlibDecoder<File>> {
//     fn set_offset(&mut self, offset: u64) -> Result<u64, AppError> {
//         let pos = match self.by_ref().bytes().nth((offset - 1) as usize) {
//             None => {
//                 return Err(AppError::App {
//                     err: AppCustomErrorKind::SeekPosBeyondEof,
//                     msg: format!("tried to set offset beyond EOF, at: {}", offset),
//                 })
//             }
//             Some(x) => x,
//         };
//         Ok(pos.unwrap() as u64)
//     }
// }

/// Trait, implemented by `LogFile` to search patterns.
pub trait Lookup {
    fn lookup(&mut self, tag: &Tag) -> Result<(), AppError>;
    fn lookup_from_reader<R: BufRead + Seeker>(
        &mut self,
        reader: R,
        tag: &Tag,
    ) -> Result<(), AppError>;
}

impl Lookup for LogFile {
    ///Just a wrapper function for a file.
    fn lookup(&mut self, tag: &Tag) -> Result<(), AppError> {
        // open target file
        let file = File::open(&self.path)?;

        // if file is compressed, we need to call a specific reader
        let reader = match self.extension.as_deref() {
            Some("gz") => {
                let decoder = GzDecoder::new(file);
                let reader = BufReader::new(decoder);
                self.lookup_from_reader(reader, tag)?;
            }
            // Some("zip") => {
            //     let decoder = ZlibDecoder::new(file);
            //     let reader = BufReader::new(decoder);
            //     self.lookup_from_reader(reader, tag, settings)?;
            // },
            Some(&_) | None => {
                let reader = BufReader::new(file);
                self.lookup_from_reader(reader, tag)?;
            }
        };

        // if self.compressed {
        //     info!("file {:?} is compressed", &self.path);
        //     let decoder = GzDecoder::new(file);
        //     let reader = BufReader::new(decoder);
        //     self.lookup_from_reader(reader, tag, settings)?;
        // } else {
        //     let reader = BufReader::new(file);
        //     self.lookup_from_reader(reader, tag, settings)?;
        // };

        //output
        Ok(())
    }

    fn lookup_from_reader<R: BufRead + Seeker>(
        &mut self,
        mut reader: R,
        tag: &Tag,
    ) -> Result<(), AppError> {
        // uses the same buffer
        let mut line = String::with_capacity(1024);

        // get rundata corresponding to tag name
        let mut rundata = self.or_insert(&tag.name);

        // initialize counters
        info!(
            "starting read from last offset={}, last line={}",
            rundata.last_offset, rundata.last_line
        );
        let mut bytes_count = rundata.last_offset;
        let mut line_number = rundata.last_line;

        // move to position if already recorded, and not rewind
        //if !tag.options.rewind && rundata.last_offset != 0 {
        if rundata.last_offset != 0 {
            reader.set_offset(rundata.last_offset)?;
        }

        loop {
            // read until \n (which is included in the buffer)
            let ret = reader.read_line(&mut line);

            // read_line() returns a Result<usize>
            match ret {
                Ok(bytes_read) => {
                    // EOF: save last file address to restart from this address for next run
                    if bytes_read == 0 {
                        //self.last_offset = reader.seek(SeekFrom::Current(0)).unwrap();
                        break;
                    }

                    // we've been reading a new line successfully
                    line_number += 1;
                    bytes_count += bytes_read as u64;
                    //println!("====> line#={}, file={}", line_number, line);

                    // check. if somethin found
                    // if let Some(caps) = tag.patterns.captures(&line) {
                    //     debug!("file {:?}, line match: {:?}", self.path, caps);
                    //     break;

                    //     // if option.script, replace capture groups and call script
                    //     // time out if any,
                    // }
                    if let Some(caps) = tag.captures(&line) {
                        debug!("line match: {:?}", caps);
                        break;
                    }

                    // reset buffer to not accumulate data
                    line.clear();
                }
                // a rare IO error could occur here
                Err(err) => {
                    return Err(AppError::Io(err));
                }
            };
        }

        // save current offset and line number
        rundata.last_offset = bytes_count;
        rundata.last_line = line_number;

        // and last run
        let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        rundata.last_run = time.as_secs();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{BufReader,Write, Read};

    use flate2::{Compression, GzBuilder, read::GzDecoder};

    use crate::error::*;
    use crate::logfile::LogFile;
    use crate::lookup::Lookup;

    use regex::Regex;

    use crate::lookup::Seeker;

    //use crate::setup::{create_ascii, create_gzip};

    // create a temporary file to test our modules
    #[allow(dead_code)]
    fn create_ascii(name: &str) -> std::path::PathBuf {
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
        f.flush().unwrap();

        ascii_file
    }

    // create a temporary gzipped file to test our BufReader
    #[allow(dead_code)]
    pub fn create_gzip(name: &str) -> std::path::PathBuf {
        let az = "ABCDEFGHIJKLMNOPQRSTUVWXYZ".repeat(2);

        let mut gzip_file = std::env::temp_dir();
        gzip_file.push(name);
        //gzip_file.set_extension("gz");

        let mut f =
            File::create(&gzip_file).expect("unable to create temporary gzip file for unit tests");
        let mut gz = GzBuilder::new()
            .write(&f, Compression::default());

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
        f.flush().unwrap();

        gzip_file
    }

    fn get_compressed_reader(file: &std::path::PathBuf) -> BufReader<GzDecoder<File>> {
        let file_name = create_gzip("clftest.gz");
        let file = File::open(file_name).expect("unable to open compressed test file");
        let decoder = GzDecoder::new(file);
        let reader = BufReader::new(decoder); 
        
        reader
    }



    #[test]
    fn test_seeker_uncompressed_file() {
        let file_name = create_ascii("clftest.txt");
        let file = File::open(file_name).expect("unable to open test file");
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
        let file_name = create_gzip("clftest.gz");
        let mut buffer = [0; 1];

        let mut reader = get_compressed_reader(&file_name);
        let mut offset = reader.set_offset(1);
        assert!(offset.is_ok());
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer[0], 'B' as u8);

        reader = get_compressed_reader(&file_name);
        offset = reader.set_offset(0);
        assert!(offset.is_ok());
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer[0], 'A' as u8);

        reader = get_compressed_reader(&file_name);
        offset = reader.set_offset(26);
        assert!(offset.is_ok());
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer[0], '\n' as u8);

        reader = get_compressed_reader(&file_name);
        offset = reader.set_offset(25);
        assert!(offset.is_ok());
        reader.read_exact(&mut buffer).unwrap();
        assert_eq!(buffer[0], 'Z' as u8);

        reader = get_compressed_reader(&file_name);
        offset = reader.set_offset(10000);
        assert!(offset.is_err());
    }
}
