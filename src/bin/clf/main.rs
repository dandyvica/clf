use clf::{config::Config, error::*, logfile::LogFile, lookup::Lookup};

mod args;
use args::CliOptions;

fn main() -> Result<(), AppError> {
    // manage arguments
    let options = CliOptions::get_options();

    // load configuration file as specified from the command line
    let config = Config::from_file(&options.config_file)?;

    // read snapshot data

    println!("{:?}", config);

    // loop through all searches
    for search in &config.searches {
        // create a LogFile struct
        let mut logfile = match LogFile::new(&search.logfile) {
            Ok(lf) => lf,
            Err(e) => panic!("error"),
        };

        // now we can search for the pattern
        logfile.lookup(&search);

        // save snapshot data

        println!("{:?}", logfile);
    }

    // write snapshot

    Ok(())
}
