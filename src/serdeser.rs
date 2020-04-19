use std::fs::File;
use std::io::BufReader;

use serde::{Deserialize, Serialize};

use crate::error::*;
use crate::logfile::LogFile;

// A trait for saving log file data, last run, etc into either
// a CSV file, a SQL databse, a JSON file, etc
pub trait SerDeser {
    //fn write(&self, source: &str);
    fn read(&self, source: &str, file: &str) -> Result<Option<LogFile>, AppError>;
}

#[derive(Serialize, Deserialize)]
pub struct RunData {
    logfile_list: Vec<LogFile>,
}

impl SerDeser for RunData {
    fn read(&self, source: &str, file_name: &str) -> Result<Option<LogFile>, AppError> {
        // open file and create a reader
        let file = File::open(file_name)?;
        let reader = BufReader::new(file);

        // read JSON data
        let run_data: RunData = serde_json::from_reader(reader)?;

        // look for the logfile data
        let logfile = run_data
            .logfile_list
            .iter()
            .find(|x| x.path.to_str().unwrap() == source);

        Ok(logfile)
    }
}

// impl SerDeser for LogFile {
//     fn serialize(&self, source: &str) {
//         // write data on CSV file
//         let data = format!("{:?};{}", self.path, self.last_pos);
//         write(source, data);
//     }

//     fn deserialize(&self, source: &str, file: &str) -> Result<LogFile, AppError> {
//         // read all data and maps to a vector
//         match read_to_string(source) {
//             // no error reading the whole file
//             Ok(data) => {
//                 // try to find a matching line
//                 for line in data.lines() {
//                     let s: Vec<_> = line.split(';').collect();
//                     if s[0] == file {
//                         return LogFile::new(s[0]);
//                     }
//                 }
//                 // if not, a new LogFile is created
//                 return LogFile::new(file);
//             }
//             Err(e) => {
//                 // if the run file is not found, start from scratch
//                 if e.kind() == ErrorKind::NotFound {
//                     return LogFile::new(file);
//                 }
//                 // another nasty error occured here
//                 else {
//                     return Err(AppError::Io(e));
//                 }
//             }
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use crate::logfile::LogFile;
    use crate::serdeser::SerDeser;

    #[test]
    fn test_csv() {
        if cfg!(target_os = "linux") {
            let lf = LogFile::new("/var/log/syslog").unwrap();
            lf.serialize("/tmp/ser.csv");
        }
    }
}
