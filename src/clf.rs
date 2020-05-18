use std::fs::OpenOptions;
use std::io::{stdin, Read};
use std::path::PathBuf;
use std::process::exit;
use std::thread;
use std::time::Duration;

#[macro_use]
extern crate log;
extern crate simplelog;
use simplelog::*;

use rclf::{
    command::ChildReturn,
    config::{Config, LogSource},
    error::AppError,
    logfile::{Lookup, Wrapper},
    snapshot::Snapshot,
    variables::Vars,
};

mod args;
use args::CliOptions;

mod error;
use error::*;

fn main() -> Result<(), AppError> {
    // create a vector of thread handles for keeping track of what we've created and
    // wait for them to finish
    let mut children_list: Vec<ChildReturn> = Vec::new();

    // manage arguments from command line
    let options = CliOptions::get_options();

    // load configuration file as specified from the command line
    // handle case of stdin input
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
            "error loading config file {:?}, error = {}",
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

    // which is the default logger ?
    let logger = &options
        .clf_logfile
        .as_deref()
        .unwrap_or(config.get_logger_name());

    // initialize logger
    match WriteLogger::init(
        LevelFilter::Debug,
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
            eprintln!("unable to create log file, error={}", e);
            exit(EXIT_LOGGER_ERROR);
        }
    };
    info!("using configuration file {:?}", &options.config_file);
    info!("options {:?}", &options);

    // create initial variables
    let mut vars = Vars::new();

    // get snapshot file file
    let snapfile = config.get_snapshot_name();

    // delete snapshot file if asked
    if options.delete_snapfile {
        if let Err(e) = std::fs::remove_file(&snapfile) {
            // not found could be a viable error
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!(
                    "unable to delete snapshot file {:?}, error={}",
                    &snapfile, e
                );
                exit(EXIT_LOGGER_ERROR);
            }
        };
        info!("deleting snapshot file {:?}", &snapfile);
    }

    // read snapshot data
    let mut snapshot = match Snapshot::load(&snapfile) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("unable to load snapshot file {:?}, error={}", &snapfile, e);
            exit(EXIT_SNAPSHOT_DELETE_ERROR);
        }
    };
    info!(
        "loaded snapshot file {:?}, data = {:?}",
        &snapfile, &snapshot
    );
    debug!("{:#?}", config);

    // loop through all searches
    for search in &config.searches {
        // log some useful info
        info!("searching for logfile={:?}", &search.logfile);

        // create a LogFile struct or get it from snapshot
        let logfile = snapshot.or_insert(&search.logfile)?;
        debug!("calling or_insert() at line {}", line!());

        // for each tag, search inside logfile
        for tag in &search.tags {
            debug!("searching for tag={}", &tag.name);

            // wraps all structures into a helper struct
            let mut wrapper = Wrapper {
                global: &config.global,
                tag: &tag,
                vars: &mut vars,
            };

            // now we can search for the pattern and save the thread handle if a script was called
            match logfile.lookup(&mut wrapper) {
                Ok(child_ret) => {
                    children_list.push(child_ret);
                }
                Err(e) => error!(
                    "error {} when searching logfile {:?} for tag {}",
                    e, &search.logfile, &tag.name
                ),
            }
        }
    }

    // write snapshot
    if let Err(e) = snapshot.save(&snapfile) {
        eprintln!("unable to save snapshot file {:?}, error={}", &snapfile, e);
        exit(EXIT_SNAPSHOT_SAVE_ERROR);
    }

    // teardown
    info!("waiting for all processes to finish");
    // for mut started_child in children_list {
    //     debug_assert!(started_child.child.is_some());

    //     let mut child = started_child.child.unwrap();

    //     let mutex = std::sync::Mutex::new(child);
    //     let arc = std::sync::Arc::new(mutex);
    //     let child_thread = thread::spawn(move || {
    //         thread::sleep(Duration::from_millis(10));
    //         let mut guard = arc.lock().unwrap();
    //         guard.kill();
    //     });
    // }

    wait_children(children_list);

    info!("end of searches");
    Ok(())
}

fn wait_children(children_list: Vec<ChildReturn>) {
    // store thread handles to wait for their job to finish
    let mut thread_handles = Vec::new();

    info!("waiting for all processes to finish");
    for started_child in children_list {
        debug_assert!(started_child.child.is_some());

        // get a mutable reference
        let mut child = started_child.child.unwrap();

        // save pid & path
        let pid = child.id();
        let path = started_child.path;

        // now if timeout has not yet occured, start a new thread to wait and kill process ??
        let elapsed = started_child.start_time.unwrap().elapsed().as_secs();

        // if timeout occured, try to kill anyway ;-)
        if elapsed > started_child.timeout {
            match child.kill() {
                Ok(_) => info!("process {} already killed", child.id()),
                Err(e) => info!("error {}", e),
            }
        }
        // else wait a little ;-)
        else {
            let mutex = std::sync::Mutex::new(child);
            let arc = std::sync::Arc::new(mutex);

            debug!("waiting for script={}, pid={} to finish", path.display(), pid);

            let child_thread = thread::spawn(move || {
                thread::sleep(Duration::from_secs(20));
                let mut guard = arc.lock().unwrap();
                guard.kill();
            });

            thread_handles.push(child_thread);
        }
    }

    // wait for thread to finish
    for handle in thread_handles {
        handle.join().expect("error waiting for thread");
    }
}
