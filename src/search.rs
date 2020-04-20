use std::io::{Read, Seek, SeekFrom};

use crate::bufreader::{ClfBufRead, ClfBufReader};
use crate::error::*;
use crate::logfile::LogFile;

trait Search {
    fn search<S, F, U>(&self, stream: S, seeker: F) -> Result<Option<U>, AppError>
    where
        F: Fn(&str) -> Option<U>,
        S: Read,
        ClfBufReader<S>: ClfBufRead + Seek;
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

impl Search for LogFile {
    fn search<S, F, U>(&self, stream: S, seeker: F) -> Result<Option<U>, AppError>
    where
        F: Fn(&str) -> Option<U>,
        S: Read,
        ClfBufReader<S>: ClfBufRead + Seek,
    {
        // uses the same buffer
        let mut line = String::with_capacity(1024);

        // create a bufreader
        let mut reader = ClfBufReader::new(stream);

        // move to position if already recorded
        reader.seek(SeekFrom::Start(self.last_pos))?;

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

                    // check
                    if let Some(output) = seeker(&line) {
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

#[cfg(test)]
mod tests {
    use std::fs::File;

    use crate::error::*;
    use crate::logfile::LogFile;
    use crate::search::Search;

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

        let output = logfile.search(file, seeker);

        assert_eq!(output.unwrap(), Some(true));
    }
}
