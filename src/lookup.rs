use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::time::SystemTime;

use crate::bufreader::{ClfBufRead, ClfBufReader};
use crate::config::Search;
use crate::error::*;
use crate::logfile::LogFile;
use crate::pattern::PatternSet;

pub trait Lookup {
    fn lookup(&mut self, search: &Search) -> Result<(), AppError>;
}

// impl Search for LogFile {
//     fn search<F, P, U>(&self, func: F, pattern: &P) -> Result<Option<U>, AppError>
//     where
//         F: Fn(&P, &str) -> Option<U>,
//     {
//         // open target file
//         let file = File::open(&self.path)?;

//         // if file is compressed, we need to call a specific reader
//         let output = if self.is_compressed() {
//             let decoder = GzDecoder::new(file);
//             let reader = BufReader::new(decoder);
//             self.reader_search(reader, func, pattern)
//         } else {
//             let reader = BufReader::new(file);
//             self.reader_search(reader, func, pattern)
//         };

//         output
//     }
// }

impl Lookup for LogFile {
    fn lookup(&mut self, search: &Search) -> Result<(), AppError> {
        // uses the same buffer
        let mut line = String::with_capacity(1024);

        // initialize counters
        let mut bytes_count = self.last_pos;
        let mut line_number = self.last_line;

        // create a bufreader
        let file = std::fs::File::open(&self.path)?;
        let mut reader = BufReader::new(file);

        // move to position if already recorded, and not rewind
        if !search.options.rewind {
            // if file is compressed, the Seek trait is not implemented. So directly move
            // to the offset
            if self.compressed {
                //reader.by_ref().bytes().nth((self.last_pos - 1) as usize)?;
            } else {
                reader.seek(SeekFrom::Start(self.last_pos))?;
            }
        }

        loop {
            // read until \n (which is included in the buffer)
            let ret = reader.read_line(&mut line);

            // read_line() returns a Result<usize>
            match ret {
                Ok(bytes_read) => {
                    // EOF: save last file address to restart from this address for next run
                    if bytes_read == 0 {
                        //self.last_pos = reader.seek(SeekFrom::Current(0)).unwrap();
                        break;
                    }

                    // we've been reading a new line successfully
                    line_number += 1;
                    bytes_count += bytes_read as u64;

                    // check. if somethin found
                    if let Some(caps) = search.patterns.captures(&line) {
                        println!("file {:?}, line match: {:?}", self.path, caps);

                        // if option.script, replace capture groups and call script
                        // time out if any,
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
        if self.compressed {
            self.last_pos = bytes_count;
        } else {
            self.last_pos = reader.seek(SeekFrom::Current(0))?;
        }
        self.last_line = line_number;

        // and last run
        let time =  SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        self.last_run = time.as_secs();

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

    #[test]
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

        //let output = logfile.search(file, seeker);

        //assert_eq!(output.unwrap(), Some(true));
    }
}
