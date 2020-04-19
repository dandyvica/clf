use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};

use regex::{Captures, Regex};

use crate::error::*;
use crate::logfile::LogFile;

trait Scan {
    // the return type of a search
    type Output;

    // the type of a pattern used to search
    type Pattern;

    fn scan<F>(
        &mut self,
        func: F,
        pattern: &Self::Pattern,
    ) -> Result<Option<Self::Output>, AppError>
    where
        F: Fn(&str, &Self::Pattern) -> Option<Self::Output>;
}

impl Scan for LogFile {
    type Output = String;
    type Pattern = Regex;

    fn scan<F>(
        &mut self,
        func: F,
        pattern: &Self::Pattern,
    ) -> Result<Option<Self::Output>, AppError>
    where
        F: Fn(&str, &Self::Pattern) -> Option<Self::Output>,
    {
        // open target file
        let file = File::open(&self.path)?;

        // uses a reader buffer
        let mut reader = BufReader::new(file);
        let mut line = String::with_capacity(1024);

        // move to position if already recorded
        reader.seek(SeekFrom::Start(self.last_pos))?;

        loop {
            let ret = reader.read_line(&mut line);

            // read_line() returns a Result<usize>
            match ret {
                Ok(bytes_read) => {
                    // EOF: save last file address to restart from this address for next run
                    if bytes_read == 0 {
                        self.last_pos = reader.seek(SeekFrom::Current(0)).unwrap();
                        break;
                    }

                    // check
                    if let Some(output) = func(&line, pattern) {
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

// #[cfg(test)]
// mod tests {
//     use crate::error::*;
//     use crate::logfile::LogFile;

//     struct Patterns {

//     }

//     #[test]
//     #[cfg(target_os = "linux")]
//     fn test_new() {
//     }
// }
