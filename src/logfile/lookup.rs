//! This is where the main function used to loop and call callback is defined.
use std::borrow::Cow;
use std::io::BufRead;
use std::time::SystemTime;

use log::{debug, error, info, trace};

use crate::misc::{
    constants::*,
    error::{AppError, AppResult},
};

use crate::configuration::{
    callback::{CallbackHandle, ChildData},
    global::GlobalOptions,
    options::SearchOptions,
    pattern::PatternCounters,
    tag::Tag,
    vars::RuntimeVars,
};

use crate::logfile::{logfile::LogFile, seeker::Seeker};

use crate::{context, prefix_var};
pub trait Lookup<T> {
    fn reader<R: BufRead + Seeker>(
        &mut self,
        reader: R,
        tag: &Tag,
        global_options: &GlobalOptions,
    ) -> AppResult<Vec<ChildData>>;
}

/// A unit struct to represent a reader which is not calling any script but just scans the logfile and outputs matched lines.
pub struct BypassReader;

/// A unit struct to represent a reader which reads each line, tests for a match and called a callback.
pub struct FullReader;

// this will call the relevant reader
#[derive(Debug, PartialEq)]
pub enum ReaderCallType {
    BypassReaderCall,
    FullReaderCall,
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
    ///     - reset thresholds if `savethresholds` is set: those thresholds trigger a callback whenever they are reached
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
        tag: &Tag,
        global_options: &GlobalOptions,
    ) -> AppResult<Vec<ChildData>> {
        //------------------------------------------------------------------------------------
        // 1. initialize local variables
        //------------------------------------------------------------------------------------
        info!(
            "start processing logfile:{} for tag:{}",
            self.id.canon_path.display(),
            tag.name
        );

        // create new reader
        //let mut reader = LogReader::from_path(&self.id.canon_path)?;
        let path = self.id.canon_path.clone();

        // uses the same buffer
        let mut buffer = Vec::with_capacity(DEFAULT_STRING_CAPACITY);

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
        let mut run_data = self.rundata_for_tag(&tag.name);
        trace!("tagname: {:?}, run_data:{:?}\n\n", &tag.name, run_data);

        // store pid: it'll be used for output message
        run_data.pid = std::process::id();

        // if we don't need to read the file from the beginning, adjust counters and set offset
        if tag.options.rewind {
            run_data.start_offset = 0;
            run_data.start_line = 0;
        } else {
            run_data.start_offset = run_data.last_offset;
            run_data.start_line = run_data.last_line;
            bytes_count = run_data.last_offset;
            line_number = run_data.last_line;

            // move to previous offset
            reader.set_offset(run_data.last_offset)?;
        }

        info!(
            "starting read from last offset={}, last line={}",
            bytes_count, line_number
        );

        // reset exec count
        run_data.counters.exec_count = 0;

        // resets thresholds if requested
        // this will count number of matches for warning & critical, to see if this matches the thresholds
        // first is warning, second is critical
        if !tag.options.savethresholds {
            run_data.counters.critical_count = 0;
            run_data.counters.warning_count = 0;
        }

        //------------------------------------------------------------------------------------
        // 3. loop to read each line of the file
        //------------------------------------------------------------------------------------
        loop {
            // reset runtime variables because they change for every line read, apart from CLF_LOGFILE
            // which is the same for each log
            //vars.retain(&[&var!("LOGFILE"), &var!("TAG")]);
            // test
            let mut vars = RuntimeVars::default();

            // read until '\n' (which is included in the buffer)
            let ret = reader.read_until(b'\n', &mut buffer);

            // truncate the line if asked
            if tag.options.truncate != 0 {
                buffer.truncate(tag.options.truncate);
            }

            // to deal with UTF-8 conversion problems, use the lossy method. It will replace non-UTF-8 chars with ?
            let mut line = String::from_utf8_lossy(&buffer);

            // remove newline
            //line.to_mut().pop();

            // delete '\n' or '\r\n' from the eol
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

                    // if stopat is reached, stop here
                    if tag.options.stopat == line_number {
                        break;
                    }

                    // do we just need to go to EOF ?
                    // TODO: implement go to EOF directly
                    if tag.options.fastforward {
                        buffer.clear();
                        continue;
                    }

                    trace!("====> line#={}, line={}", line_number, line);

                    // is there a match, regarding also exceptions?
                    if let Some(pattern_match) = tag.is_match(&line) {
                        debug!(
                            "found a match tag={}, line={}, line#={}, re=({:?},{}), warning_count={}, critical_count={}",
                            tag.name,
                            line.clone(),
                            line_number,
                            pattern_match.pattern_type,
                            pattern_match.regex.as_str(),
                            run_data.counters.warning_count,
                            run_data.counters.critical_count
                        );

                        // when a threshold is reached, give up
                        if !run_data.is_threshold_reached(
                            &pattern_match.pattern_type,
                            tag.options.criticalthreshold,
                            tag.options.warningthreshold,
                        ) {
                            buffer.clear();
                            continue;
                        }

                        // if we've been asked to trigger the script, first add relevant variables
                        if tag.options.runcallback {
                            // create variables which will be set as environment variables when script is called
                            vars.insert_var(
                                prefix_var!("LOGFILE"),
                                path.to_str().unwrap_or("error converting PathBuf"),
                            );
                            vars.insert_var(prefix_var!("TAG"), &tag.name);
                            let ln = format!("{}", line_number);
                            vars.insert_var(prefix_var!("LINE_NUMBER"), &ln);
                            vars.insert_var(prefix_var!("LINE"), &line);
                            vars.insert_var(
                                prefix_var!("MATCHED_RE"),
                                pattern_match.regex.as_str(),
                            );
                            let pattern_type = String::from(pattern_match.pattern_type);
                            vars.insert_var(prefix_var!("MATCHED_RE_TYPE"), &pattern_type);

                            // insert number of captures and capture groups
                            let nb_caps = vars.insert_captures(pattern_match.regex, &line);
                            let nb_caps_s = nb_caps.to_string();
                            vars.insert_var(prefix_var!("NB_CG"), &nb_caps_s);

                            debug!("added variables: {:?}", vars);

                            // now call script if upper run limit is not reached yet
                            if run_data.counters.exec_count < tag.options.runlimit {
                                // in case of a callback error, stop iterating and save state here
                                match tag.callback_call(
                                    Some(&global_options.script_path),
                                    &global_options.global_vars,
                                    &vars,
                                    &mut handle,
                                ) {
                                    Ok(child) => {
                                        // save child structure
                                        if let Some(c) = child {
                                            children.push(c);
                                        }

                                        // increment number of script executions or number of JSON data sent
                                        run_data.counters.exec_count += 1;
                                    }
                                    Err(e) => {
                                        error!(
                                            "error <{}> when calling callback <{:#?}>",
                                            e, tag.callback
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
                    early_ret = Some(AppError::from_error(
                        e,
                        &format!("error reading logfile {:?} at line {}", &path, line_number),
                    ));
                    break;
                }
            };
        }

        // save current offset and line number
        run_data.last_offset = bytes_count;
        run_data.last_line = line_number;

        // and last run
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| context!(e, "error calculating durations",))?;
        run_data.last_run = time.as_secs_f64();
        run_data.last_run_secs = time.as_secs();

        // criticalthreshold or warning thresholds are set, need to reflect reality for error counts
        // need to test against thresholds in case of high values
        counters_calculation(&mut run_data.counters, &tag.options);

        info!(
            "number of callback execution: {}",
            run_data.counters.exec_count
        );
        trace!(
            "========================> end processing logfile:{} for tag:{}",
            self.id.canon_path.display(),
            tag.name
        );

        // return error if we got one or the list of children from calling the script
        match early_ret {
            None => Ok(children),
            Some(e) => Err(e),
        }
    }
}

// manage error counters depending on options
fn counters_calculation(counters: &mut PatternCounters, options: &SearchOptions) {
    // do we need to save our thresholds ?
    if options.savethresholds {
        // critical errors
        if options.criticalthreshold != 0 {
            if counters.critical_count < options.criticalthreshold {
                // nothing to do
            } else {
                // or just the delta
                counters.critical_count -= options.criticalthreshold;
            }
        }
        // warning errors
        if options.warningthreshold != 0 {
            // warning errors
            if counters.warning_count < options.warningthreshold {
                // nothing to do
            } else {
                // or just the delta
                counters.warning_count -= options.warningthreshold;
            }
        }
    } else {
        // critical errors
        if options.criticalthreshold != 0 {
            if counters.critical_count < options.criticalthreshold {
                // no errors in this case
                counters.critical_count = 0;
            } else {
                // or just the delta
                counters.critical_count -= options.criticalthreshold;
            }
        }
        // warning errors
        if options.warningthreshold != 0 {
            // warning errors
            if counters.warning_count < options.warningthreshold {
                // no errors in this case
                counters.warning_count = 0;
            } else {
                // or just the delta
                counters.warning_count -= options.warningthreshold;
            }
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
        tag: &Tag,
        _global_options: &GlobalOptions,
    ) -> AppResult<Vec<ChildData>> {
        for (line_number, line) in reader.lines().enumerate() {
            let text = {
                if let Err(e) = line {
                    error!(
                        "error {} reading logfile {} using BypassReader",
                        e,
                        &self.id.canon_path.display()
                    );
                    return Err(AppError::from_error(
                        e,
                        &format!(
                            "error reading logfile {:?} at line {}",
                            self.id.canon_path, line_number
                        ),
                    ));
                }
                line.unwrap()
            };

            // is there a match ?
            if let Some(pattern_match) = tag.is_match(&text) {
                // print out also captures
                let mut vars = RuntimeVars::default();
                vars.insert_captures(pattern_match.regex, &text);

                // cap0 is the whole match, no need to keep it as the full line is printed anyway
                vars.retain(|k, _| k != &String::from("CLF_CAPTURE0"));

                eprintln!(
                    "{}:{}:{}:{}:[{}]:{}",
                    &self.id.canon_path.display(),
                    &tag.name,
                    String::from(pattern_match.pattern_type),
                    line_number,
                    vars,
                    text
                );
            }
        }

        Ok(Vec::new())
    }
}
