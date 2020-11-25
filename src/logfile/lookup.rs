//! This is where the main function used to loop and call callback is defined.
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::BufRead;
use std::time::SystemTime;

use log::{debug, error, info, trace};

use crate::misc::{error::AppError, util::Cons};

use crate::config::{
    callback::{CallbackHandle, ChildData},
    pattern::PatternType,
    vars::RuntimeVars,
};

use crate::logfile::{
    logfile::{LogFile, Wrapper},
    seeker::Seeker,
};

use crate::prefix_var;

pub trait Lookup<T> {
    fn reader<R: BufRead + Seeker>(
        &mut self,
        reader: R,
        wrapper: &mut Wrapper,
    ) -> Result<Vec<ChildData>, AppError>;
}

// In this case, the logfile is only read and callback not called at all
struct BypassReader;

// The regular reader
struct FullReader;

// this will call the relevant reader
#[derive(Debug, PartialEq)]
pub enum ReaderCallType {
    BypassReaderCall,
    FullReaderCall,
}

impl ReaderCallType {
    pub fn call<R: BufRead + Seeker>(
        &self,
        log: &mut LogFile,
        reader: R,
        wrapper: &mut Wrapper,
    ) -> Result<Vec<ChildData>, AppError> {
        match self {
            ReaderCallType::BypassReaderCall => {
                Lookup::<BypassReader>::reader(log, reader, wrapper)
            }
            ReaderCallType::FullReaderCall => Lookup::<FullReader>::reader(log, reader, wrapper),
        }
    }
}

impl Lookup<FullReader> for LogFile {
    /// The main function of the whole process. Reads a logfile and tests for each line if it matches the regexes.
    ///
    /// Detailed design:
    ///
    /// 1. initialize local variables
    ///     - buffer which will hold read data from each line
    ///     - a `Child` vector which will receive its value from the optional call to a spawned script
    ///     - line and bytes read counters whichkeep track of current line and current number of bytes read
    ///
    /// 2. reset `RunData` fields depending on local options
    ///     - get a mutable reference on `RunData` structure
    ///     - reset thresholds if `savethresholdcount` is set: those thresholds trigger a callback whenever they are reached
    ///     - set current file pointers (offset and line number) to the last ones recorded in the `RunData` structure. If local option
    ///       is set to `rewind`, read from the beginning of the file and set offsets accordingly
    ///
    /// 3. loop to read each line of the file
    ///     - read a line as a byte Vec and convert (lossy) to UTF-8
    ///     - test if each line matches a pattern
    ///     - if yes:
    ///         - test if thresholds are reached. If not loop
    ///         - add rumtime variables, only related to the current line, pattern etc
    ///         - if a script is defined to be called, call the script and save the `Child` return structure
    fn reader<R: BufRead + Seeker>(
        &mut self,
        mut reader: R,
        wrapper: &mut Wrapper,
    ) -> Result<Vec<ChildData>, AppError> {
        //------------------------------------------------------------------------------------
        // 1. initialize local variables
        //------------------------------------------------------------------------------------
        info!(
            "start processing logfile:{} for tag:{}",
            self.path.display(),
            wrapper.tag.name()
        );

        // create new reader
        //let mut reader = LogReader::from_path(&self.path)?;
        let path = self.path.clone();

        // uses the same buffer
        let mut buffer = Vec::with_capacity(Cons::DEFAULT_STRING_CAPACITY);

        // define a new child handle. This is an Option because the script couldn't be called if not requested so
        let mut children = Vec::new();

        // initialize line & byte counters
        let mut bytes_count = 0;
        let mut line_number = 0;

        // to keep handles: stream etc
        let mut handle = CallbackHandle::default();

        // sometimes, early return due to callback errors or I/O errors
        let mut early_ret: Option<AppError> = None;

        //------------------------------------------------------------------------------------
        // 2. reset `RunData` fields depending on local options
        //------------------------------------------------------------------------------------

        // get run_data corresponding to tag name, or insert that new one if not yet in the snapshot file
        let mut run_data = self.rundata_for_tag(&wrapper.tag.name());
        trace!(
            "tagname: {:?}, run_data:{:?}\n\n",
            &wrapper.tag.name(),
            run_data
        );

        // if we don't need to read the file from the beginning, adjust counters and set offset
        if !wrapper.tag.options.rewind {
            bytes_count = run_data.last_offset;
            line_number = run_data.last_line;
            reader.set_offset(run_data.last_offset)?;
        }

        info!(
            "starting read from last offset={}, last line={}",
            bytes_count, line_number
        );

        // reset exec count
        run_data.exec_count = 0;

        //------------------------------------------------------------------------------------
        // 3. loop to read each line of the file
        //------------------------------------------------------------------------------------
        loop {
            // reset runtime variables because they change for every line read, apart from CLF_LOGFILE
            // which is the same for each log
            //wrapper.vars.retain(&[&var!("LOGFILE"), &var!("TAG")]);
            // test
            let mut vars = RuntimeVars::default();

            // read until '\n' (which is included in the buffer)
            let ret = reader.read_until(b'\n', &mut buffer);

            // to deal with UTF-8 conversion problems, use the lossy method. It will replace non-UTF-8 chars with ?
            let mut line = String::from_utf8_lossy(&buffer);

            // remove newline
            //line.to_mut().pop();

            // delete '\n' or '\r\n' form the eol
            LogFile::purge_line(&mut line);

            // and line feed for Windows platforms
            // #[cfg(target_family = "windows")]
            // line.to_mut().pop();

            // read_line() returns a Result<usize>
            match ret {
                Ok(bytes_read) => {
                    // EOF: save last file address to restart from this address for next run
                    if bytes_read == 0 {
                        break;
                    }

                    // we've been reading a new line successfully
                    line_number += 1;
                    bytes_count += bytes_read as u64;

                    // do we just need to go to EOF ?
                    if wrapper.tag.options.fastforward {
                        buffer.clear();
                        continue;
                    }

                    trace!("====> line#={}, line={}", line_number, line);

                    // is there a match, regarding also exceptions?
                    if let Some(pattern_match) = wrapper.tag.is_match(&line) {
                        debug!(
                            "found a match tag={}, line={}, line#={}, re=({:?},{}), warning_count={}, critical_count={}",
                            wrapper.tag.name(),
                            line.clone(),
                            line_number,
                            pattern_match.pattern_type,
                            pattern_match.regex.as_str(),
                            run_data.warning_count,
                            run_data.critical_count
                        );

                        // increments thresholds and compare with possible defined limits and accumulate counters for plugin output
                        match pattern_match.pattern_type {
                            PatternType::critical => {
                                run_data.critical_count += 1;
                                if run_data.critical_count < wrapper.tag.options.criticalthreshold {
                                    buffer.clear();
                                    continue;
                                }
                                //wrapper.global_counter.critical_count += 1;
                                wrapper.logfile_counter.critical_count += 1;
                            }
                            PatternType::warning => {
                                run_data.warning_count += 1;
                                if run_data.warning_count < wrapper.tag.options.warningthreshold {
                                    buffer.clear();
                                    continue;
                                }
                                //wrapper.global_counter.warning_count += 1;
                                wrapper.logfile_counter.warning_count += 1;
                            }
                            // this special Ok pattern resets counters
                            PatternType::ok => {
                                run_data.critical_count = 0;
                                run_data.warning_count = 0;

                                // no need to process further: don't call a script
                                buffer.clear();
                                continue;
                            }
                        };

                        // if we've been asked to trigger the script, first add relevant variables
                        if wrapper.tag.options.runcallback {
                            // create variables which will be set as environment variables when script is called
                            vars.insert_var(
                                prefix_var!("LOGFILE"),
                                path.to_str().unwrap_or("error converting PathBuf"),
                            );
                            vars.insert_var(prefix_var!("TAG"), wrapper.tag.name());
                            let ln = format!("{}", line_number);
                            vars.insert_var(prefix_var!("LINE_NUMBER"), &ln);
                            vars.insert_var(prefix_var!("LINE"), &line);
                            vars.insert_var(
                                prefix_var!("MATCHED_RE"),
                                pattern_match.regex.as_str(),
                            );
                            let pattern_type = String::from(pattern_match.pattern_type);
                            vars.insert_var(prefix_var!("MATCHED_RE_TYPE"), &pattern_type);

                            vars.insert_captures(pattern_match.regex, &line);

                            debug!("added variables: {:?}", vars);

                            // now call script if upper run limit is not reached yet
                            if run_data.exec_count < wrapper.tag.options.runlimit {
                                // in case of a callback error, stop iterating and save state here
                                match wrapper.tag.callback_call(
                                    Some(&wrapper.global.path()),
                                    wrapper.global.user_vars(),
                                    &vars,
                                    &mut handle,
                                ) {
                                    Ok(child) => {
                                        // save child structure
                                        if child.is_some() {
                                            children.push(child.unwrap());
                                        }

                                        // increment number of script executions or number of JSON data sent
                                        run_data.exec_count += 1;
                                    }
                                    Err(e) => {
                                        error!(
                                            "error <{}> when calling callback <{:#?}>",
                                            e,
                                            wrapper.tag.callback()
                                        );
                                        early_ret = Some(e);
                                        break;
                                    }
                                };
                            }
                        };
                    }

                    // reset buffer to not accumulate data
                    buffer.clear();
                }
                // a rare IO error could occur here
                Err(e) => {
                    error!("read_line() error kind: {:?}, line: {}", e.kind(), line);
                    early_ret = Some(AppError::Io(e));
                    break;
                }
            };
        }

        // save current offset and line number
        run_data.last_offset = bytes_count;
        run_data.last_line = line_number;

        // resets thresholds if requested
        // this will count number of matches for warning & critical, to see if this matches the thresholds
        // first is warning, second is critical
        if !wrapper.tag.options.savethresholdcount {
            run_data.critical_count = 0;
            run_data.warning_count = 0;
        }

        // and last run
        let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;
        run_data.last_run = time.as_secs_f64();

        info!("number of callback execution: {}", run_data.exec_count);
        trace!("logfile_counter: {:?}", &wrapper.logfile_counter);
        trace!(
            "========================> end processing logfile:{} for tag:{}",
            self.path.display(),
            wrapper.tag.name()
        );

        // return error if we got one or the list of children from calling the script
        if early_ret.is_some() {
            Err(early_ret.unwrap())
        } else {
            Ok(children)
        }
    }
}

impl Lookup<BypassReader> for LogFile {
    /// In this case, the reader just read each line and prints out the lines matching the regexes.
    /// No computation of counters in made
    /// TODO: add line number
    fn reader<R: BufRead + Seeker>(
        &mut self,
        reader: R,
        wrapper: &mut Wrapper,
    ) -> Result<Vec<ChildData>, AppError> {
        for (line_number, line) in reader.lines().enumerate() {
            let text = {
                if let Err(e) = line {
                    error!(
                        "error {} reading logfile {} using BypassReader",
                        e,
                        &self.path.display()
                    );
                    return Err(AppError::Io(e));
                }
                line.unwrap()
            };

            // is there a match ?
            if let Some(pattern_match) = wrapper.tag.is_match(&text) {
                // print out also captures: this helps to verify regexes
                // TODO: add captures
                let caps = pattern_match.regex.captures(&text).unwrap();

                // we'll store here all captures with their matched text
                let mut dict: HashMap<String, &str> = HashMap::new();

                // now loop and get text corresponding to either name or position
                for (i, cg_name) in pattern_match.regex.capture_names().enumerate() {
                    match cg_name {
                        None => {
                            if let Some(m) = caps.get(i) {
                                // variable will be: CLF_CAPTURE2 (example)
                                dict.insert(format!("cap{}", i), m.as_str());
                            }
                        }
                        Some(cap_name) => {
                            if let Some(m) = caps.name(cap_name) {
                                // variable will be: CLF_FOO (example)
                                dict.insert(cap_name.to_string(), m.as_str());
                            }
                        }
                    }
                }

                // cap0 is the whole match, no need to keep it as the full line is printed anyway
                dict.retain(|k, _| k != &String::from("cap0"));

                eprintln!(
                    "<{}>:<{}>:<{}>:<{}>:<{:?}>:<{}>",
                    &self.path.display(),
                    &wrapper.tag.name(),
                    String::from(pattern_match.pattern_type),
                    line_number,
                    dict,
                    text
                );
            }
        }

        Ok(Vec::new())
    }
}
