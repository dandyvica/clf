use std::path::PathBuf;
use std::str::FromStr;

use clap::{App, Arg};
use simplelog::LevelFilter;

//use crate::error::AppExitCode;
use crate::logfile::lookup::ReaderCallType;
use crate::misc::{
    nagios::{Nagios, NagiosVersion},
    util::Cons,
};

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
    pub snapshot_file: Option<PathBuf>,
    pub reader_type: ReaderCallType,
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
            max_logger_size: Cons::MAX_LOGGER_SIZE * 1024 * 1024,
            show_options: false,
            nagios_version: NagiosVersion::Nrpe3,
            snapshot_file: None,
            reader_type: ReaderCallType::FullReaderCall,
        }
    }
}

impl CliOptions {
    pub fn options() -> CliOptions {
        let matches = App::new("Log files reader")
            .version("0.1")
            .author("Alain Viguier dandyvica@gmail.com")
            .about(r#"A log file checker inspired by the Nagios check_logfiles plugin. Checklogfiles (clf) will try to detect some regex patterns in logfiles specified in a YAML configuration file.

            Project home page: https://github.com/dandyvica/clf
            
            "#)
            .arg(
                Arg::new("config")
                    .long_about("Mandatory argument. The name and path of the YAML configuration file, containing logfiles to search for and patterns to match.")
                    .short('c')
                    .long("config")
                    .required(true)
                    .long_about("Name of the YAML configuration file.")
                    .takes_value(true),
            )
            .arg(
                Arg::new("logger")
                    .short('l')
                    .long("logger")
                    .required(false)
                    .long_about("Name of the logger file for logging information of this executable.")
                    .takes_value(true),
            )
            .arg(
                Arg::new("delete-snapshot")
                    .short('d')
                    .long("delete-snapshot")
                    .required(false)
                    .long_about("Delete snapshot file before searching.")
                    .takes_value(false),
            )
            .arg(
                Arg::new("syntax-check")
                    .short('s')
                    .long("syntax-check")
                    .required(false)
                    .long_about("Check configuration file correctness, print it out and exit.")
                    .takes_value(false),
            )
            .arg(
                Arg::new("loglevel")
                    .short('g')
                    .long("loglevel")
                    .required(false)
                    .long_about("When logger is enabled, set the minimum logger level. Defaults to 'Info'.")
                    .possible_values(&["Off", "Error", "Warn", "Info", "Debug", "Trace"])
                    .takes_value(true),
            )
            .arg(
                Arg::new("max-logsize")
                    .short('m')
                    .long("max-logsize")
                    .required(false)
                    .long_about("When logger is enabled, set the maximum logger size (in Mb). If specified, logger file will be deleted first if current size is over this value. Defaults to 50 MB.")
                    .takes_value(true),
            )
            .arg(
                Arg::new("show-options")
                    .short('o')
                    .long("show-options")
                    .required(false)
                    .long_about("Just show the command line options passed and exit.")
                    .takes_value(false),
            )
            .arg(
                Arg::new("nagios-version")
                    .short('n')
                    .long("nagios-version")
                    .required(false)
                    .long_about("Set the Nagios NRPE protocol version used for plugin output. Default to version 3.")
                    .possible_values(&["2", "3"])
                    .takes_value(true),
            )
            .arg(
                Arg::new("no-callback")
                    .short('a')
                    .long("no-callback")
                    .required(false)
                    .long_about("Don't run any callback, just read all logfiles in the configuration file. Used to check whether regexes are correct.")
                    .takes_value(false),
            )
            .arg(
                Arg::new("snapshot")
                    .short('p')
                    .long("snapshot")
                    .required(false)
                    .long_about("Override the snapshot file specified in the configuration file. It will default to the platform-dependent name using the temporary directory if not provided in configuration file or by using this flag.")
                    .takes_value(true),
            )
            .get_matches();

        // save all cli options into a structure
        let mut options = CliOptions::default();

        // config file is mandatory. Try to canonicalize() at the same time.
        let config_file = PathBuf::from(matches.value_of("config").unwrap());
        let canonicalized_config_file = config_file.canonicalize();
        if let Err(ref e) = canonicalized_config_file {
            Nagios::exit_critical(&format!(
                "error trying to canonicalize config file: {}, error: {}",
                config_file.display(),
                e
            ));
        }
        options.config_file = canonicalized_config_file.unwrap();

        // optional logger file
        if matches.is_present("logger") {
            options.clf_logger = PathBuf::from(matches.value_of("logger").unwrap());
        }

        // optional check for reading
        if matches.is_present("no-callback") {
            options.reader_type = ReaderCallType::BypassReaderCall;
        }

        // other options too
        options.check_conf = matches.is_present("syntax-check");
        options.delete_snapfile = matches.is_present("delete-snapshot");
        options.show_options = matches.is_present("show-options");

        // if matches.is_present("loglevel") {
        //     options.logger_level =
        //         LevelFilter::from_str(matches.value_of("loglevel").unwrap()).unwrap();
        // }
        options.logger_level = matches.value_of_t("loglevel").unwrap_or(LevelFilter::Info);

        // if matches.is_present("nagios-version") {
        //     options.nagios_version =
        //         NagiosVersion::from_str(matches.value_of("nagios-version").unwrap()).unwrap();
        // }
        options.nagios_version = matches
            .value_of_t("nagios-version")
            .unwrap_or(NagiosVersion::Nrpe3);

        if matches.is_present("snapshot") {
            options.snapshot_file = Some(PathBuf::from(matches.value_of("snapshot").unwrap()));
        }

        options.max_logger_size = matches
            .value_of_t("max-logsize")
            .unwrap_or(Cons::MAX_LOGGER_SIZE * 1024 * 1024);

        // if matches.is_present("max-logsize") {
        //     options.max_logger_size = value_t!(matches, "max-logsize", u64)
        //         .unwrap_or(Cons::MAX_LOGGER_SIZE)
        //         * 1024
        //         * 1024;
        // }

        options.show_options = matches.is_present("show-options");
        if options.show_options {
            // print out options if requested and exits
            Nagios::exit_ok(&format!("{:#?}", options));
        }

        options
    }
}
