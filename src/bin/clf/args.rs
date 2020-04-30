use std::path::PathBuf;

use clap::{App, Arg};

use clf::error::AppError;

// This structure holds the command line arguments
#[derive(Debug, Default)]
pub struct CliOptions {
    pub config_file: PathBuf,
    pub settings_file: Option<PathBuf>,
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
            .author("Alain Viguier dandyvica@airfrance.fr")
            .about("A log file checker inspired by the Nagios check_logfiles plugin")
            .arg(
                Arg::with_name("config")
                    .short("c")
                    .long("config")
                    .required(true)
                    .help("Name of the YAML configuration file")
                    .takes_value(true),
            )
            .arg(
                Arg::with_name("settings")
                    .short("s")
                    .long("settings")
                    .required(false)
                    .help("Name of the optional settings file")
                    .takes_value(true),
            )
            // .arg(
            //     Arg::with_name("pbar")
            //         .short("p")
            //         .long("pbar")
            //         .required(false)
            //         .help("Shows a progress bar when reading the input file")
            //         .takes_value(false),
            // )
            // .arg(
            //     Arg::with_name("stats")
            //         .short("s")
            //         .long("stats")
            //         .required(false)
            //         .help("Outputs some stats after reading file is over")
            //         .takes_value(false),
            // )
            // .arg(
            //     Arg::with_name("limit")
            //         .short("l")
            //         .long("limit")
            //         .required(false)
            //         .help("Stops converting after number of lines reaches the limit specified")
            //         .takes_value(true),
            // )
            // .arg(
            //     Arg::with_name("output")
            //         .short("o")
            //         .long("output")
            //         .required(false)
            //         .help("Name of the output file. If not specified, outputs to standard output")
            //         .takes_value(true),
            // )
            // .arg(
            //     Arg::with_name("debug")
            //         .short("d")
            //         .long("debug")
            //         .required(false)
            //         .help("If set, just output YAML data and command line arguments and exits")
            //         .takes_value(false),
            // )
            // .arg(
            //     Arg::with_name("list")
            //         .short("l")
            //         .long("list")
            //         .required(false)
            //         .help("Comma separated list of object types to be saved")
            //         .takes_value(true),
            // )
            // .arg(
            //     Arg::with_name("noout")
            //         .short("s")
            //         .long("noout")
            //         .required(false)
            //         .help("Suppress all output information, including converted clapi data")
            //         .takes_value(false),
            // )
            .get_matches();

        // save all cli options into a structure
        let mut options = CliOptions::default();

        // config file is mandatory
        options.config_file = PathBuf::from(matches.value_of("config").unwrap());

        // settings file is optional
        if let Some(settings) = matches.value_of("settings") {
            options.settings_file = Some(PathBuf::from(settings));
        }

        options
    }
}
