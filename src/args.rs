use std::path::PathBuf;
use std::str::FromStr;

use clap::{App, Arg};
use simplelog::LevelFilter;

// This structure holds the command line arguments
#[derive(Debug)]
pub struct CliOptions {
    pub config_file: PathBuf,
    pub clf_logfile: Option<PathBuf>,
    pub delete_snapfile: bool,
    pub check_conf: bool,
    pub log_level: LevelFilter,
}

/// Implements `Default` trait for `CliOptions`.
impl Default for CliOptions {
    fn default() -> Self {
        CliOptions {
            config_file: PathBuf::default(),
            clf_logfile: None,
            delete_snapfile: false,
            check_conf: false,
            log_level: LevelFilter::Error,
        }
    }
}

impl CliOptions {
    // pub fn new() -> CliOptions {
    //     CliOptions {
    //         config_file: "".to_string(),
    //         // input_file: "".to_string(),
    //         // output_file: "".to_string(),
    //         // progress_bar: false,
    //         // stats: false,
    //         // limit: 0usize,
    //         // debug: false,
    //         // ot_list: Vec::new(),
    //         // output_allowed: true,
    //     }
    // }

    pub fn get_options() -> CliOptions {
        let matches = App::new("Log files reader")
            .version("0.1")
            .author("Alain Viguier dandyvica@gmail.com")
            .about("A log file checker inspired by the Nagios check_logfiles plugin")
            .arg(
                Arg::with_name("config")
                    .long_help("The name and path of the YAML configuration file, containing logfiles to search for and patterns to match.")
                    .short("c")
                    .long("config")
                    .required(true)
                    .help("Name of the YAML configuration file. Use dash '-' for standard input")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("clflog")
                    .short("l")
                    .long("clflog")
                    .required(false)
                    .help("Name of the log file for logging information")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("delsnap")
                    .short("d")
                    .long("delsnap")
                    .required(false)
                    .help("Delete snapshot file before searching")
                    .takes_value(false),
            )
            .arg(
                Arg::with_name("chkcnf")
                    .short("n")
                    .long("checkconf")
                    .required(false)
                    .help("Check configuration file correctness, print it out and exit")
                    .takes_value(false),
            )
            .arg(
                Arg::with_name("loglevel")
                    .short("g")
                    .long("loglevel")
                    .required(false)
                    .help("When logger is enabled, sets the minimum logger level. Possible values are: Error, Warn, Info, Debug, Trace. Defaults to Error.")
                    .possible_values(&["Error", "Warn", "Info", "Debug", "Trace"])
                    .takes_value(true),
            )
            .get_matches();

        // save all cli options into a structure
        let mut options = CliOptions::default();

        // config file is mandatory
        options.config_file = PathBuf::from(matches.value_of("config").unwrap());

        // logfile file is optional
        // options.clf_logfile = match matches.value_of("clflog") {
        //     Some(log) => Some(PathBuf::from(log)),
        //     None => None,
        // };

        options.clf_logfile = matches
            .value_of("clflog")
            .and_then(|log| Some(PathBuf::from(log)));

        // other options too
        options.check_conf = matches.is_present("chkcnf");
        options.delete_snapfile = matches.is_present("delsnap");

        options.log_level = matches
            .value_of("loglevel")
            .map_or(LevelFilter::Error, |opt| {
                LevelFilter::from_str(opt).unwrap()
            });

        options
    }
}
