use log::{debug, info, trace};
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
    callback::ChildReturn,
    config::{Config, LogSource},
    error::AppError,
    logfile::{Lookup, Wrapper},
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
    let mut children_list: Vec<ChildReturn> = Vec::new();

    // manage arguments from command line
    let options = CliOptions::get_options();

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
            .set_time_format("%H:%M:%S.%f".to_string())
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
    eprintln!("using logger file: {}", logger.display());
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
        if let Err(e) = &search.logfile.is_usable() {
            error!(
                "logfile: {} is not a file or is not accessible, error: {}",
                &search.logfile.display(),
                e
            );
            continue;
        }

        // create a LogFile struct or get it from snapshot
        let logfile = snapshot.or_insert(&search.logfile)?;
        debug!("calling or_insert() at line {}", line!());

        // for each tag, search inside logfile
        for tag in &search.tags {
            debug!("searching for tag: {}", &tag.name);

            // wraps all structures into a helper struct
            let mut wrapper = Wrapper {
                global: config.get_global(),
                tag: &tag,
                vars: &mut vars,
            };

            // now we can search for the pattern and save the child handle if a script was called
            match logfile.lookup(&mut wrapper) {
                Ok(child_ret) => {
                    // add child only is has really been started
                    if child_ret.is_some() {
                        children_list.push(child_ret.unwrap());
                    }
                }
                Err(e) => {
                    error!(
                        "error: {} when searching logfile: {} for tag: {}",
                        e,
                        &search.logfile.display(),
                        &tag.name
                    );
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
    info!("waiting for all processes to finish");
    wait_children(children_list);

    info!(
        "end of searches, elapsed: {} seconds",
        now.elapsed().as_secs_f32()
    );

    // print out final results
    Ok(())
}

/// Manage end of all started processes from clf.
fn wait_children(children_list: Vec<ChildReturn>) {
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
            Err(e) => println!("error attempting to wait: {} for pid:{}", e, pid),
        };
    }

    // wait for thread to finish
    for handle in thread_handles {
        handle.join().expect("error waiting for thread");
    }
}
