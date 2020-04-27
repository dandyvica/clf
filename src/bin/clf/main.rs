use clf::config::Config;
use clf::error::*;
use clf::logfile::LogFile;

mod args;
use args::CliOptions;

fn main() -> Result<(), AppError> {
    // manage arguments
    let options = CliOptions::get_options();

    // load configuration file as specified from the command line
    let config = Config::from_file(&options.config_file)?;

    println!("{:?}", config);

    // loop through all searches
    for search in &config.searches {
        // create a LogFile struct
        let logfile = match LogFile::new(&search.logfile) {
            Ok(lf) => lf,
            Err(e) => panic!("error"),
        };

        // now we can search for the pattern

        println!("{:?}", logfile);
    }

    Ok(())
}
