use std::path::PathBuf;

use clap::{App, Arg};

// This structure holds the command line arguments
#[derive(Debug, Default)]
pub struct CliOptions {
    pub config_file: PathBuf,
    pub clf_logfile: PathBuf,
    pub delete_snapfile: bool,
    pub check_conf: bool,
    // pub input_file: String,
    // pub output_file: String,
    // pub progress_bar: bool,
    // pub stats: bool,
    // pub limit: usize,
    // pub debug: bool,
    // pub ot_list: Vec<String>,
    // pub output_allowed: bool,
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
            .get_matches();

        // save all cli options into a structure
        let mut options = CliOptions::default();

        // config file is mandatory
        options.config_file = PathBuf::from(matches.value_of("config").unwrap());

        // logfile file is optional
        options.clf_logfile = match matches.value_of("clflog") {
            Some(log) => PathBuf::from(log),
            None => {
                let mut dir = std::env::temp_dir();
                dir.push("clf.log");
                dir
            }
        };

        options.check_conf = matches.is_present("chkcnf");

        options
    }
}
