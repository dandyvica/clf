use std::fs::{OpenOptions,File};
use std::io::ErrorKind;
use std::thread;

#[macro_use]
extern crate log;
extern crate simplelog;
use simplelog::*;

use clf::{
    config::Config, error::AppError, logfile::LogFile, lookup::Lookup, settings::Settings,
    snapshot::Snapshot,
};

mod args;
use args::CliOptions;

fn main() -> Result<(), AppError> {
    // create a vector of thread handles for keep track of what we've created and
    // wait for them to finish
    let mut handle_list: Vec<thread::JoinHandle<()>> = Vec::new();

    // manage arguments from command line
    let options = CliOptions::get_options();

    // initialize logger
    match WriteLogger::init(
        LevelFilter::Debug,
        simplelog::Config::default(),
        OpenOptions::new().append(true).create(true).open(options.clf_logfile).unwrap(),
    ) {
        Ok(_) => (),
        Err(e) => panic!("unable to create log file"),
    };

    // load configuration file as specified from the command line
    let config = match Config::from_file(&options.config_file) {
        Ok(conf) => conf,
        Err(e) => {
            error!(
                "error loading config file {:?}, error = {}",
                &options.config_file, e
            );
            std::process::exit(1);
        }
    };

    // load the optional settings
    let settings: Option<Settings> = match options.settings_file {
        None => None,
        Some(f) => Some(Settings::from_file(f)?),
    };

    // read snapshot data
    let mut snapshot = match Snapshot::load("/tmp/clf.snapshot") {
        Ok(s) => s,
        Err(e) => panic!("error {:?}", e),
    };

    println!("{:?}", config);

    // loop through all searches
    for search in &config.searches {
        // log some useful info
        info!("searching for log={:?}", &search.logfile);

        // create a LogFile struct or get it from snapshot
        let mut logfile = snapshot.get_mut_or_insert(&search.logfile)?;

        //let mut logfile = snapshot.get_mut(&search.logfile).unwrap();

        // let mut logfile = match LogFile::new(&search.logfile) {
        //     Ok(lf) => lf,
        //     Err(e) => panic!("error {:?}", e),
        // };

        // now we can search for the pattern
        logfile.lookup(&search, settings.as_ref());

        // save snapshot data

        println!("{:?}", logfile);
    }

    // write snapshot
    snapshot.save("/tmp/clf.snapshot")?;

    // teardown
    info!("waiting for all threads to finish");
    for handle in handle_list {
        handle.join();
    }

    info!("end of searches");
    Ok(())
}
