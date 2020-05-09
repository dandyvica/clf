use std::fs::{File, OpenOptions};
use std::io::ErrorKind;
use std::thread;

#[macro_use]
extern crate log;
extern crate simplelog;
use simplelog::*;

//extern crate rclf;
use rclf::{
    config::{Config, Tag},
    error::AppError,
    logfile::LogFile,
    lookup::Lookup,
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
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(options.clf_logfile)
            .unwrap(),
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
    info!("using configuration file {:?}", &options.config_file);

    // get snapshot file file
    let snapfile = Snapshot::default_name();

    // delete snapshot file if asked
    if options.delete_snapfile {
        std::fs::remove_file(&snapfile)?;
    }

    // load the optional settings
    // let settings: Option<Settings> = match options.settings_file {
    //     None => None,
    //     Some(f) => Some(Settings::from_file(f)?),
    // };

    // read snapshot data
    let mut snapshot = match Snapshot::load(&snapfile) {
        Ok(s) => s,
        Err(e) => panic!("error {:?}", e),
    };

    println!("{:?}", config);

    // loop through all searches
    for search in &config.searches {
        // log some useful info
        info!("searching for log={:?}", &search.logfile);

        // create a LogFile struct or get it from snapshot
        let mut logfile = snapshot.or_insert(&search.logfile)?;

        // for each tag, search inside logfile
        for tag in &search.tags {
            // insert new rundata if not present or get ref on it
            //let rundata = logfile.or_insert(tag.name);

            // now we can search for the pattern
            logfile.lookup(&tag)?;
        }
    }

    // write snapshot
    snapshot.save(&snapfile)?;

    // teardown
    info!("waiting for all threads to finish");
    for handle in handle_list {
        handle.join();
    }

    info!("end of searches");
    Ok(())
}
