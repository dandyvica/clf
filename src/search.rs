use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};

use flate2::read::GzDecoder;
use regex::{Captures, Regex};

use crate::error::*;
use crate::logfile::LogFile;
use crate::search::reader_search::ReaderSearch;

trait Search {
    fn search<F, P, U>(&self, func: F, pattern: &P) -> Result<Option<U>, AppError>
    where
        F: Fn(&P, &str) -> Option<U>;
}

impl Search for LogFile {
    fn search<F, P, U>(&self, func: F, pattern: &P) -> Result<Option<U>, AppError>
    where
        F: Fn(&P, &str) -> Option<U>,
    {
        // open target file
        let file = File::open(&self.path)?;

        // if file is compressed, we need to call a specific reader
        let output = if self.is_compressed() {
            let decoder = GzDecoder::new(file);
            let reader = BufReader::new(decoder);
            self.reader_search(reader, func, pattern)
        } else {
            let reader = BufReader::new(file);
            self.reader_search(reader, func, pattern)
        };

        output
    }
}

mod reader_search {

    use crate::error::*;
    use crate::logfile::LogFile;
    use std::io::BufRead;

    pub trait ReaderSearch {
        fn reader_search<T, F, P, U>(
            &self,
            reader: T,
            func: F,
            pattern: &P,
        ) -> Result<Option<U>, AppError>
        where
            T: BufRead,
            F: Fn(&P, &str) -> Option<U>;
    }

    impl ReaderSearch for LogFile {
        fn reader_search<T, F, P, U>(
            &self,
            mut reader: T,
            func: F,
            pattern: &P,
        ) -> Result<Option<U>, AppError>
        where
            T: BufRead,
            F: Fn(&P, &str) -> Option<U>,
        {
            // uses a reader buffer

            let mut line = String::with_capacity(1024);

            // move to position if already recorded
            //reader.seek(SeekFrom::Start(self.last_pos))?;

            loop {
                let ret = reader.read_line(&mut line);

                // read_line() returns a Result<usize>
                match ret {
                    Ok(bytes_read) => {
                        // EOF: save last file address to restart from this address for next run
                        if bytes_read == 0 {
                            //self.last_pos = reader.seek(SeekFrom::Current(0)).unwrap();
                            break;
                        }

                        // check
                        if let Some(output) = func(pattern, &line) {
                            return Ok(Some(output));
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

            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error::*;
    use crate::logfile::LogFile;
    use crate::search::Search;

    use regex::Regex;

    struct SearchPattern {
        critical: Vec<Regex>,
        warning: Vec<Regex>,
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_search1() {
        let logfile = LogFile::new("./tests/files/access.log");
        let re = Regex::new("^83").unwrap();
        let match_func = |re: &Regex, s: &str| Some(re.is_match(s));
        let output: Result<Option<bool>, AppError> = logfile.search(match_func, &re);

        assert_eq!(output.unwrap(), Some(true));
    }
}
