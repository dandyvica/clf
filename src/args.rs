use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;

use clap::{App, Arg};
use simplelog::LevelFilter;

use crate::error::AppExitCode;
use rclf::nagios::NagiosVersion;

/// We define here the maximum size for the logger file (in Mb).
const MAX_LOGGER_SIZE: u64 = 10 * 1024 * 1024;

/// This structure holds the command line arguments.
#[derive(Debug)]
pub struct CliOptions {
    pub config_file: PathBuf,
    pub clf_logger: PathBuf,
    pub delete_snapfile: bool,
    pub check_conf: bool,
    pub logger_level: LevelFilter,
    pub max_logger_size: u64,
    pub show_options: bool,
    pub nagios_version: NagiosVersion,
}

/// Implements `Default` trait for `CliOptions`.
impl Default for CliOptions {
    fn default() -> Self {
        // build a default logger file
        let mut default_logger = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
        default_logger.push("clf.log");

        CliOptions {
            config_file: PathBuf::default(),
            clf_logger: default_logger,
            delete_snapfile: false,
            check_conf: false,
            logger_level: LevelFilter::Info,
            max_logger_size: MAX_LOGGER_SIZE,
            show_options: false,
            nagios_version: NagiosVersion::NagiosNrpe3,
        }
    }
}

impl CliOptions {
    pub fn get_options() -> CliOptions {
        let matches = App::new("Log files reader")
            .version("0.1")
            .author("Alain Viguier dandyvica@gmail.com")
            .about(r#"A log file checker inspired by the Nagios check_logfiles plugin. Checklogfiles (clf) will try to detect some regex patterns in logfiles specified in a YAML configuration file.

            Project home page: https://github.com/dandyvica/clf
            
            "#)
            .arg(
                Arg::with_name("config")
                    .long_help("Mandatory argument. The name and path of the YAML configuration file, containing logfiles to search for and patterns to match.")
                    .short("c")
                    .long("config")
                    .required(true)
                    .help("Name of the YAML configuration file. Use dash '-' for standard input.")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("clflog")
                    .short("l")
                    .long("clflog")
                    .required(false)
                    .help("Name of the logger file for logging information of this process.")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("delsnap")
                    .short("d")
                    .long("delsnap")
                    .required(false)
                    .help("Delete snapshot file before searching.")
                    .takes_value(false),
            )
            .arg(
                Arg::with_name("chkcnf")
                    .short("e")
                    .long("checkconf")
                    .required(false)
                    .help("Check configuration file correctness, print it out and exit.")
                    .takes_value(false),
            )
            .arg(
                Arg::with_name("loglevel")
                    .short("g")
                    .long("loglevel")
                    .required(false)
                    .help("When logger is enabled, set the minimum logger level. Defaults to 'Info'.")
                    .possible_values(&["Off", "Error", "Warn", "Info", "Debug", "Trace"])
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("logsize")
                    .short("m")
                    .long("logsize")
                    .required(false)
                    .help("When logger is enabled, set the maximum logger size (in Mb). If specified, logger file will be deleted if current size is over this value.")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("showopt")
                    .short("s")
                    .long("showopt")
                    .required(false)
                    .help("Just show the command line options passed and exit.")
                    .takes_value(false),
            )
            .arg(
                Arg::with_name("nagver")
                    .short("n")
                    .long("nagver")
                    .required(false)
                    .help("Set the Nagios NRPE protocol version used for plugin output. Defaults to version 3.")
                    .possible_values(&["2", "3"])
                    .takes_value(true),
            )
            .get_matches();

        // save all cli options into a structure
        let mut options = CliOptions::default();

        // config file is mandatory
        options.config_file = PathBuf::from(matches.value_of("config").unwrap());

        // options.clf_logger = matches
        //     .value_of("clflog")
        //     .and_then(|log| Some(PathBuf::from(log)));
        // optional logger file
        if matches.is_present("clflog") {
            options.clf_logger = PathBuf::from(matches.value_of("clflog").unwrap());
        }

        // other options too
        options.check_conf = matches.is_present("chkcnf");
        options.delete_snapfile = matches.is_present("delsnap");
        options.show_options = matches.is_present("showopt");

        if matches.is_present("loglevel") {
            options.logger_level =
                LevelFilter::from_str(matches.value_of("loglevel").unwrap()).unwrap();
        }

        if matches.is_present("nagver") {
            options.nagios_version =
                NagiosVersion::from_str(matches.value_of("nagver").unwrap()).unwrap();
        }

        if matches.is_present("logsize") {
            let value = matches.value_of("logsize").unwrap();
            let size = match value.parse::<u64>() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!(
                        "error converting value logsize: {} into an integer, error: {}",
                        value, e
                    );
                    exit(AppExitCode::ERROR_CONV as i32);
                }
            };
            options.max_logger_size = size * 1024 * 1024;
        }

        options
    }
}
