use std::io::ErrorKind;

use clf::{config::Config, error::*, logfile::LogFile, lookup::Lookup, snapshot::Snapshot};

mod args;
use args::CliOptions;

fn main() -> Result<(), AppError> {
    // manage arguments
    let options = CliOptions::get_options();

    // load configuration file as specified from the command line
    let config = Config::from_file(&options.config_file)?;

    // read snapshot data
    let mut snapshot = match Snapshot::load("/tmp/clf.snapshot") {
        Ok(s) => s,
        Err(e) => panic!("error {:?}", e),
    };

    println!("{:?}", config);

    // loop through all searches
    for search in &config.searches {
        // create a LogFile struct or get it from snapshot
        let mut logfile = snapshot.get_mut_or_insert(&search.logfile)?;

        //let mut logfile = snapshot.get_mut(&search.logfile).unwrap();

        // let mut logfile = match LogFile::new(&search.logfile) {
        //     Ok(lf) => lf,
        //     Err(e) => panic!("error {:?}", e),
        // };

        // now we can search for the pattern
        logfile.lookup(&search);

        // save snapshot data

        println!("{:?}", logfile);
    }

    // write snapshot
    snapshot.save("/tmp/clf.snapshot")?;

    Ok(())
}
