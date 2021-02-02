//! Manage command line arguments here.
use std::path::PathBuf;

use clap::{App, Arg};
use simplelog::LevelFilter;

use crate::logfile::lookup::ReaderCallType;
use crate::misc::extension::Expect;
use crate::misc::{
    nagios::{Nagios, NagiosVersion},
    util::*,
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
    pub tera_context: Option<String>,
    pub extra_vars: Option<Vec<String>>,
    pub show_rendered: bool,
    pub reset_log: bool,
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
            max_logger_size: MAX_LOGGER_SIZE * 1024 * 1024,
            show_options: false,
            nagios_version: NagiosVersion::Nrpe3,
            snapshot_file: None,
            reader_type: ReaderCallType::FullReaderCall,
            tera_context: None,
            extra_vars: None,
            show_rendered: false,
            reset_log: false,
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
                    .long_about("Mandatory argument. The name and path of the YAML configuration file, containing logfiles to search for and patterns to match")
                    .short('c')
                    .long("config")
                    .required(true)
                    .long_about("Name of the YAML configuration file")
                    .takes_value(true),
            )
            .arg(
                Arg::new("log")
                    .short('l')
                    .long("log")
                    .required(false)
                    .long_about("Name of the log file for logging information of this executable. Not to be confused with the logfile to search into")
                    .takes_value(true),
            )
            .arg(
                Arg::new("delete-snapshot")
                    .short('d')
                    .long("delete-snapshot")
                    .required(false)
                    .long_about("Delete snapshot file before searching")
                    .takes_value(false),
            )
            .arg(
                Arg::new("syntax-check")
                    .short('s')
                    .long("syntax-check")
                    .required(false)
                    .long_about("Check configuration file correctness, print it out and exit")
                    .takes_value(false),
            )
            .arg(
                Arg::new("log-level")
                    .short('g')
                    .long("log-level")
                    .required(false)
                    .long_about("When log is enabled, set the minimum log level. Defaults to 'Info'")
                    .possible_values(&["Off", "Error", "Warn", "Info", "Debug", "Trace"])
                    .takes_value(true),
            )
            .arg(
                Arg::new("max-logsize")
                    .short('m')
                    .long("max-logsize")
                    .required(false)
                    .long_about("When log is enabled, set the maximum log size (in Mb). If specified, log file will be deleted first if current size is over this value. Defaults to 50 MB")
                    .takes_value(true),
            )
            .arg(
                Arg::new("show-options")
                    .short('o')
                    .long("show-options")
                    .required(false)
                    .long_about("Just show the command line options passed and exit")
                    .takes_value(false),
            )
            .arg(
                Arg::new("show-rendered")
                    .short('w')
                    .long("show-rendered")
                    .required(false)
                    .long_about("Render the configuration file through Jinja2/Tera and exit. This is meant to check Tera substitutions")
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
                    .long_about("Don't run any callback, just read all logfiles in the configuration file and print out matching line. Used to check whether regexes are correct")
                    .takes_value(false),
            )
            .arg(
                Arg::new("snapshot")
                    .short('p')
                    .long("snapshot")
                    .required(false)
                    .long_about("Override the snapshot file specified in the configuration file. It will default to the platform-dependent name using the temporary directory if not provided in configuration file or by using this flag")
                    .takes_value(true),
            )
            .arg(
                Arg::new("context")
                    .short('x')
                    .long("context")
                    .required(false)
                    .long_about("A JSON string used to set the Tera context. Only valid if the tera feature is enabled")
                    .takes_value(true),
            )
            .arg(
                Arg::new("var")
                    .short('v')
                    .long("var")
                    .required(false)
                    .long_about("An optional variable to send to the defined callback, with syntax: 'var:value'. Multiple values are possible")
                    .multiple(true)
                    .takes_value(true),
            )
            .arg(
                Arg::new("overwrite-log")
                    .short('r')
                    .long("overwrite-log")
                    .required(false)
                    .long_about("Overwrite clf log if specified")
                    .takes_value(false),
            )
            .get_matches();

        // save all cli options into a structure
        let mut options = CliOptions::default();

        // config file is mandatory. Try to canonicalize() at the same time.
        let config_file = PathBuf::from(matches.value_of("config").unwrap());

        options.config_file = config_file.canonicalize().expect_critical(&format!(
            "error trying to canonicalize config file: {}",
            config_file.display()
        ));

        // optional log file
        if matches.is_present("log") {
            options.clf_logger = PathBuf::from(matches.value_of("log").unwrap());
        }

        // optional check for reading
        if matches.is_present("no-callback") {
            options.reader_type = ReaderCallType::BypassReaderCall;
        }

        // other options too
        options.check_conf = matches.is_present("syntax-check");
        options.delete_snapfile = matches.is_present("delete-snapshot");
        options.show_options = matches.is_present("show-options");
        options.show_rendered = matches.is_present("show-rendered");
        options.reset_log = matches.is_present("overwrite-log");

        options.logger_level = matches.value_of_t("log-level").unwrap_or(LevelFilter::Info);

        options.nagios_version = matches
            .value_of_t("nagios-version")
            .unwrap_or(NagiosVersion::Nrpe3);

        if matches.is_present("snapshot") {
            options.snapshot_file = Some(PathBuf::from(matches.value_of("snapshot").unwrap()));
        }

        options.max_logger_size = matches
            .value_of_t("max-logsize")
            .unwrap_or(MAX_LOGGER_SIZE * 1024 * 1024);

        options.tera_context = matches.value_of("context").map(|x| x.to_string());

        if matches.is_present("var") {
            let vars: Vec<&str> = matches.values_of("var").unwrap().collect();
            options.extra_vars = Some(vars.iter().map(|x| x.to_string()).collect());
        }

        options.show_options = matches.is_present("show-options");
        if options.show_options {
            // print out options if requested and exits
            Nagios::exit_ok(&format!("{:#?}", options));
        }

        options
    }
}
