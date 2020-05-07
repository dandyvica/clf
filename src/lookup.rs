//! 
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
use crate::settings::Settings;

pub trait Seeker {
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
        let pos = match self.by_ref().bytes().nth((offset - 1) as usize) {
            None => {
                return Err(AppError::App {
                    err: AppCustomErrorKind::SeekPosBeyondEof,
                    msg: format!("tried to set offset beyond EOF, at: {}", offset),
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

pub trait Lookup {
    fn lookup(&mut self, tag: &Tag) -> Result<(), AppError>;
    fn lookup_from_reader<R: BufRead + Seeker>(
        &mut self,
        reader: R,
        tag: &Tag,
    ) -> Result<(), AppError>;
}

impl Lookup for LogFile {
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

    use crate::error::*;
    use crate::logfile::LogFile;
    use crate::lookup::Lookup;

    use regex::Regex;

    use crate::setup::{create_ascii, create_gzip};

    struct SearchPattern {
        critical: Vec<Regex>,
        warning: Vec<Regex>,
    }

    //#[test]
    #[cfg(target_os = "linux")]
    fn test_search_file() {
        // create tmp file
        let ascii_file = create_ascii("az_ascii_search.txt");
        let file = File::open(&ascii_file).unwrap();

        // create LogFile struct
        let logfile = LogFile::new(ascii_file);

        // seeker function
        fn seeker(s: &str) -> Option<bool> {
            let re = Regex::new("^A").unwrap();
            Some(re.is_match(s))
        }

        //let output = logfile.tag(file, seeker);

        //assert_eq!(output.unwrap(), Some(true));
    }
}
