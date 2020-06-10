use log::{debug, info};
use std::fs::OpenOptions;
use std::io::{stdin, ErrorKind, Read};
use std::path::PathBuf;
use std::process::exit;
use std::thread;
use std::time::{Duration, Instant};

#[macro_use]
extern crate log;
extern crate simplelog;
use simplelog::*;

use rclf::{
    callback::ChildData,
    config::{Config, LogSource},
    error::AppError,
    logfile::{Lookup, Wrapper},
    nagios::{LogfileMatchCounter, MatchCounter, NagiosVersion},
    snapshot::Snapshot,
    util::Usable,
    variables::Variables,
};

mod args;
use args::CliOptions;

mod error;
use error::*;

fn main() -> Result<(), AppError> {
    // tick time
    let now = Instant::now();

    // create a vector of thread handles for keeping track of what we've created and
    // wait for them to finish
    let mut children_list: Vec<ChildData> = Vec::new();

    // manage arguments from command line
    let options = CliOptions::get_options();

    // this will keep cumulative number of critical or warning matches and will be used for plugin output
    let mut global_exit_counter: MatchCounter = MatchCounter::default();

    // and this for each invididual file
    let mut logfile_exit_counter = LogfileMatchCounter::new();

    // print out options if requested and exits
    if options.show_options {
        eprintln!("{:#?}", options);
        exit(EXIT_SHOW_OPTIONS);
    }

    // builds the logger from cli or the default one from platform specifics
    //let default_logger = default_logger();
    let logger = &options.clf_logger;

    //---------------------------------------------------------------------------------------------------
    // initialize logger
    // first get level filter from cli
    //---------------------------------------------------------------------------------------------------
    match WriteLogger::init(
        options.logger_level,
        simplelog::ConfigBuilder::new()
            .set_time_format("%Y-%b-%d %H:%M:%S.%f".to_string())
            .build(),
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(logger)
            .unwrap(),
    ) {
        Ok(_) => (),
        Err(e) => {
            eprintln!(
                "unable to create log file: {}, error: {}",
                logger.display(),
                e
            );
            exit(EXIT_LOGGER_ERROR);
        }
    };

    // useful traces
    info!("using configuration file: {:?}", &options.config_file);
    info!("options: {:?}", &options);

    //---------------------------------------------------------------------------------------------------
    // load configuration file as specified from the command line
    // handle case of stdin input
    //---------------------------------------------------------------------------------------------------
    let _config = if options.config_file == PathBuf::from("-") {
        let mut buffer = String::with_capacity(1024);
        let stdin = stdin();
        let mut handle = stdin.lock();

        if let Err(e) = handle.read_to_string(&mut buffer) {
            eprintln!("error reading stdin: {}", e);
            exit(EXIT_STDIN_ERROR);
        }

        Config::<LogSource>::from_str(&buffer)
    } else {
        Config::<LogSource>::from_file(&options.config_file)
    };

    // check for loading errors
    if let Err(e) = _config {
        eprintln!(
            "error loading config file: {:?}, error: {}",
            &options.config_file, e
        );
        exit(EXIT_CONFIG_ERROR);
    }

    // replace, if any, "loglist" by "logfile"
    let config = Config::<PathBuf>::from(_config.unwrap());

    // print out config if requested and exit
    if options.check_conf {
        println!("{:#?}", config);
        exit(EXIT_CONFIG_CHECK);
    }

    //---------------------------------------------------------------------------------------------------
    // create initial variables, both user & runtime
    //---------------------------------------------------------------------------------------------------
    let mut vars = Variables::new();
    vars.insert_uservars(config.get_user_vars());

    // get snapshot file file
    let snapfile = config.get_snapshot_name();

    // delete snapshot file if asked
    if options.delete_snapfile {
        if let Err(e) = std::fs::remove_file(&snapfile) {
            // 'not found' could be a viable error
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "unable to delete snapshot file: {:?}, error: {}",
                    &snapfile, e
                );
                exit(EXIT_LOGGER_ERROR);
            }
        };
        info!("deleting snapshot file {:?}", &snapfile);
    }

    //---------------------------------------------------------------------------------------------------
    // read snapshot data from file
    //---------------------------------------------------------------------------------------------------
    let mut snapshot = match Snapshot::load(&snapfile) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "unable to load snapshot file: {:?}, error: {}",
                &snapfile, e
            );
            exit(EXIT_SNAPSHOT_DELETE_ERROR);
        }
    };
    info!(
        "loaded snapshot file {:?}, data = {:?}",
        &snapfile, &snapshot
    );
    debug!("{:#?}", config);

    //---------------------------------------------------------------------------------------------------
    // loop through all searches
    //---------------------------------------------------------------------------------------------------
    for search in &config.searches {
        // log some useful info
        info!(
            "------------ searching into logfile: {}",
            &search.logfile.display()
        );

        // checks if logfile is accessible. If not, no need to move further
        if let Err(err) = &search.logfile.is_usable() {
            error!(
                "logfile: {} is not a file or is not accessible, error: {}",
                &search.logfile.display(),
                err
            );

            // report missing logfile
            // get a mutable reference on inner counter structure
            let mut logfile_counter = &mut logfile_exit_counter.or_default(&search.logfile);
            logfile_counter.app_error = (search.io_error.clone(), Some(err.to_string()));

            continue;
        }

        // create a LogFile struct or get it from snapshot
        let logfile = snapshot.or_insert(&search.logfile)?;
        debug!("calling or_insert() at line {}", line!());

        // for each tag, search inside logfile
        for tag in &search.tags {
            debug!("searching for tag: {}", &tag.name);

            // if we've been explicitely asked to not process this logfile, just loop
            if !&tag.process {
                continue;
            }

            // wraps all structures into a helper struct
            let mut wrapper = Wrapper {
                global: config.get_global(),
                tag: &tag,
                vars: &mut vars,
                global_exit_counter: &mut global_exit_counter,
                logfile_exit_counter: &mut logfile_exit_counter,
            };

            // now we can search for the pattern and save the child handle if a script was called
            match logfile.lookup(&mut wrapper) {
                // script might be started, giving back a `Child` structure with process features like pid etc
                Ok(mut children) => {
                    // merge list of children
                    if children.len() != 0 {
                        children_list.append(&mut children);
                    }
                }

                // otherwise, an error when opening (most likely) the file and then report an error on counters
                Err(err) => {
                    error!(
                        "error: {} when searching logfile: {} for tag: {}",
                        err,
                        &search.logfile.display(),
                        &tag.name
                    );

                    // get a mutable reference on inner counter structure
                    // let mut logfile_counter = &mut logfile_exit_counter.or_default(&search.logfile);
                    // logfile_counter.app_error = (tag.options.logfilemissing.clone(), Some(err));
                }
            }
        }
    }

    // write snapshot
    debug!("saving snapshot file {}", &snapfile.display());
    if let Err(e) = snapshot.save(&snapfile, config.get_snapshot_retention()) {
        eprintln!(
            "unable to save snapshot file: {:?}, error: {}",
            &snapfile, e
        );
        exit(EXIT_SNAPSHOT_SAVE_ERROR);
    }

    // teardown
    info!(
        "waiting for all processes to finish, nb of children: {}",
        children_list.len()
    );
    wait_children(children_list);

    info!(
        "end of searches, elapsed: {} seconds",
        now.elapsed().as_secs_f32()
    );

    // display output to comply to Nagios plug-in convention
    debug!("global exit counters: {:?}", global_exit_counter);
    debug!("logfile exit counters: {:?}", logfile_exit_counter);

    nagios_output(
        &global_exit_counter,
        &logfile_exit_counter,
        &options.nagios_version,
    );

    // print out final results
    Ok(())
}

/// Manage end of all started processes from clf.
fn wait_children(children_list: Vec<ChildData>) {
    // just wait a little for all commands to finish. Otherwise, the last process will not be considered to be finished.
    if !children_list.is_empty() {
        let wait_timeout = std::time::Duration::from_millis(1000);
        thread::sleep(wait_timeout);
    }

    // store thread handles to wait for their job to finish
    let mut thread_handles = Vec::new();

    for started_child in children_list {
        debug_assert!(started_child.child.is_some());

        // get a mutable reference
        let mut child = started_child.child.unwrap();

        // save pid & path
        let pid = child.id();
        let path = started_child.path;

        debug!(
            "managing end of process, pid:{}, path:{}",
            pid,
            path.display()
        );

        // use try_wait() to check if command has exited
        match child.try_wait() {
            // child has already exited. So check output status code if any
            Ok(Some(status)) => info!(
                "command with path: {}, pid: {} exited with: {}",
                path.display(),
                pid,
                status
            ),

            // child has not exited. Spawn a new thread to wait at most the timeout defined
            Ok(None) => {
                debug!("========> None");

                // now if timeout has not yet occured, start a new thread to wait and kill process ??
                let elapsed = started_child.start_time.unwrap().elapsed().as_secs();

                // if timeout occured, try to kill anyway ;-)
                if elapsed > started_child.timeout {
                    match child.kill() {
                        Ok(_) => info!("process {} killed", child.id()),
                        Err(e) => {
                            if e.kind() == ErrorKind::InvalidInput {
                                info!("process {} already killed", child.id());
                            } else {
                                info!(
                                    "error:{} trying to kill process pid:{}, path: {}",
                                    e,
                                    pid,
                                    path.display()
                                );
                            }
                        }
                    }
                } else {
                    // wait a little and spawn a new thread to kill the command
                    let mutex = std::sync::Mutex::new(child);
                    let arc = std::sync::Arc::new(mutex);

                    // we'll wait at least the remaining seconds
                    let secs_to_wait = started_child.timeout - elapsed;

                    debug!(
                        "waiting for script: {}, pid: {} to finish",
                        path.display(),
                        pid
                    );

                    let child_thread = thread::spawn(move || {
                        thread::sleep(Duration::from_secs(secs_to_wait));
                        let mut guard = arc.lock().unwrap();

                        match guard.kill() {
                            Ok(_) => info!("process {} killed", guard.id()),
                            Err(e) => {
                                if e.kind() == ErrorKind::InvalidInput {
                                    info!("process {} already killed", guard.id());
                                } else {
                                    info!(
                                        "error:{} trying to kill process pid:{}, path: {}",
                                        e,
                                        pid,
                                        path.display()
                                    );
                                }
                            }
                        }
                    });

                    thread_handles.push(child_thread);
                }
            }

            // unlikely error
            Err(e) => eprintln!("error attempting to wait: {} for pid:{}", e, pid),
        };
    }

    // wait for thread to finish
    for handle in thread_handles {
        handle.join().expect("error waiting for thread");
    }
}

/// Manage Nagios output, depending on the NRPE version.
fn nagios_output(
    global_counter: &MatchCounter,
    logfile_counter: &LogfileMatchCounter,
    nagios_version: &NagiosVersion,
) {
    // first, test if an I/O error has been detected for a logfile in any of the processed logfile.
    // if so, the overall result will boil down to the first error detected
    if let Some(logfile_in_error) = logfile_counter.iter().find(|io| io.1.app_error.1.is_some()) {
        println!("{}|", logfile_in_error.1.output().0);
    } else {
        // get global exit data, because its printed out anyway
        let global_exit_data = global_counter.output();
        println!("{}|", global_exit_data.0);
    }

    match nagios_version {
        NagiosVersion::NagiosNrpe3 => {
            for (path, counter) in logfile_counter.iter() {
                let logfile_exit_data = counter.output();
                println!("{}: {}", path.display(), logfile_exit_data.0);
            }
        }

        NagiosVersion::NagiosNrpe2 => {}
    };
}
