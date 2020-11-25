use std::path::PathBuf;
use std::str::FromStr;

use clap::{value_t, App, Arg};
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
                    .help("When logger is enabled, set the maximum logger size (in Mb). If specified, logger file will be deleted if current size is over this value. Defaults to 50 MB.")
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
            .arg(
                Arg::with_name("no-call")
                    .short("a")
                    .long("no-call")
                    .required(false)
                    .help("Don't run any callback, just read all logfiles in the configuration file. Used to check whether regexes are correct.")
                    .takes_value(false),
            )
            .arg(
                Arg::with_name("snapfile")
                    .short("p")
                    .long("snapfile")
                    .required(false)
                    .help("Overrides the snapshot file specified in the configuration file. Will defaults to the platform-dependent temporary directory in not provided in configuration file or using this flag.")
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
        if matches.is_present("clflog") {
            options.clf_logger = PathBuf::from(matches.value_of("clflog").unwrap());
        }

        // optional check for reading
        if matches.is_present("no-call") {
            options.reader_type = ReaderCallType::BypassReaderCall;
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

        if matches.is_present("snapfile") {
            options.snapshot_file = Some(PathBuf::from(matches.value_of("snapfile").unwrap()));
        }

        if matches.is_present("logsize") {
            options.max_logger_size =
                value_t!(matches, "logsize", u64).unwrap_or(Cons::MAX_LOGGER_SIZE) * 1024 * 1024;
        }

        options
    }
}
