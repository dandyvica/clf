use std::fs::OpenOptions;
use std::io::{stdin, Read};
use std::path::PathBuf;
use std::process::exit;
use std::thread;

#[macro_use]
extern crate log;
extern crate simplelog;
use simplelog::*;

use rclf::{
    config::{Config, LogSource},
    error::AppError,
    logfile::Lookup,
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
    let mut handle_list: Vec<thread::JoinHandle<()>> = Vec::new();

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

            // now we can search for the pattern and save the thread handle if a script was called
            match logfile.lookup(&tag, &mut vars) {
                Ok(try_handle) => {
                    if let Some(handle) = try_handle {
                        handle_list.push(handle);
                    }
                }
                Err(e) => error!(
                    "error {} when searching logfile {:?} for tag {}",
                    e, &search.logfile, &tag.name
                ),
            }
            // if let Some(handle) = logfile.lookup(&tag, &mut vars)? {
            //     handle_list.push(handle);
            // }
        }
    }

    // write snapshot
    if let Err(e) = snapshot.save(&snapfile) {
        eprintln!("unable to save snapshot file {:?}, error={}", &snapfile, e);
        exit(EXIT_SNAPSHOT_SAVE_ERROR);
    }

    // teardown
    info!("waiting for all threads to finish");
    for handle in handle_list {
        handle.join().expect("could join thread handle");
    }

    info!("end of searches");
    Ok(())
}
