use std::fs::OpenOptions;
//use std::io::ErrorKind;
use std::thread;

#[macro_use]
extern crate log;
extern crate simplelog;
use simplelog::*;

//extern crate rclf;
use rclf::{config::Config, error::AppError, logfile::Lookup, snapshot::Snapshot, variables::Vars};

mod args;
use args::CliOptions;

fn main() -> Result<(), AppError> {
    // create a vector of thread handles for keep track of what we've created and
    // wait for them to finish
    let mut handle_list: Vec<thread::JoinHandle<()>> = Vec::new();

    // manage arguments from command line
    let options = CliOptions::get_options();

    // load configuration file as specified from the command line
    let config = match Config::from_file(&options.config_file) {
        Ok(conf) => conf,
        Err(e) => {
            eprintln!(
                "error loading config file {:?}, error = {}",
                &options.config_file, e
            );
            std::process::exit(1);
        }
    };

    // print out config if requested and exit
    if options.check_conf {
        println!("{:#?}", config);
        std::process::exit(101);
    }

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
        Err(e) => eprintln!("unable to create log file, error={}", e),
    };
    info!("using configuration file {:?}", &options.config_file);

    // create initial variables
    let mut vars = Vars::new();

    // get snapshot file file
    let snapfile = Snapshot::default_name();

    // delete snapshot file if asked
    if options.delete_snapfile {
        std::fs::remove_file(&snapfile)?;
    }

    // read snapshot data
    let mut snapshot = match Snapshot::load(&snapfile) {
        Ok(s) => s,
        Err(e) => panic!("error {:?}", e),
    };

    debug!("{:#?}", config);

    // loop through all searches
    for search in &config.searches {
        // log some useful info
        info!("searching for log={:?}", &search.logfile);

        // create a LogFile struct or get it from snapshot
        let logfile = snapshot.or_insert(&search.logfile)?;

        // for each tag, search inside logfile
        for tag in &search.tags {
            debug!("searching for tag={}", &tag.name);

            // now we can search for the pattern and save the thread handle if a script was called
            if let Some(handle) = logfile.lookup(&tag, &mut vars)? {
                handle_list.push(handle);
            }
        }
    }

    // write snapshot
    snapshot.save(&snapfile)?;

    // teardown
    info!("waiting for all threads to finish");
    for handle in handle_list {
        handle.join().expect("could join thread handle");
    }

    info!("end of searches");
    Ok(())
}
