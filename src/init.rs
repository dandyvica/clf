//! All preliminary steps to prepare reading files
use std::fs::OpenOptions;
use std::path::PathBuf;

use simplelog::*;

use crate::args::CliOptions;
use crate::config::config::{Config, LogSource};
use crate::logfile::snapshot::Snapshot;
use crate::misc::nagios::Nagios;

/// Create a new config struct
pub fn init_config(options: &CliOptions) -> Config<PathBuf> {
    let _config = Config::<LogSource>::from_file(&options.config_file);

    // check for loading errors
    if let Err(ref e) = _config {
        // break down errors
        // match error.get_ioerror() {
        //     Some(_) => exit(AppExitCode::CONFIG_IO_ERROR as i32),
        //     None => exit(AppExitCode::CONFIG_ERROR as i32),
        // };
        //exit(AppExitCode::CONFIG_ERROR as i32);

        Nagios::exit_critical(&format!(
            "error loading config file: {:?}, error: {}",
            &options.config_file, e
        ));
    }

    // replace, if any, "loglist" by "logfile"
    let config = Config::<PathBuf>::from(_config.unwrap());
    config
    //debug!("{:#?}", config);
}

/// Create new logger and optionally delete logfile is bigger than cli value
pub fn init_log(options: &CliOptions) {
    // builds the logger from cli or the default one from platform specifics
    let logger = &options.clf_logger;

    // initialize logger
    match WriteLogger::init(
        options.logger_level,
        simplelog::ConfigBuilder::new()
            .set_time_format("%Y-%b-%d %H:%M:%S.%f".to_string())
            .build(),
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(logger)
            .unwrap(),
    ) {
        Ok(_) => (),
        Err(e) => {
            Nagios::exit_critical(&format!(
                "unable to create log file: {}, error: {}",
                logger.display(),
                e
            ));
        }
    };

    // check if we have to delete the log, because it's bigger than max logger size
    let metadata = std::fs::metadata(&logger);
    if let Err(e) = &metadata {
        Nagios::exit_critical(&format!("error on metadata() API: {}", e));
    }

    debug!(
        "current logger size is: {} bytes",
        metadata.as_ref().unwrap().len()
    );
    if metadata.as_ref().unwrap().len() > options.max_logger_size {
        if let Err(e) = std::fs::remove_file(&logger) {
            // 'not found' could be a viable error
            if e.kind() != std::io::ErrorKind::NotFound {
                error!("unable to delete logger file: {:?}, error: {}", &logger, e);
            }
        } else {
            info!("deleting logger file {:?}", &logger);
        }
    }

    // useful traces
    info!("using configuration file: {:?}", &options.config_file);
    info!("options: {:?}", &options);
}

/// Load the snapshot file
pub fn load_snapshot(options: &CliOptions) -> Snapshot {
    let snapfile = Snapshot::build_name(&options.config_file);

    // delete snapshot file if requested
    if options.delete_snapfile {
        if let Err(e) = std::fs::remove_file(&snapfile) {
            // 'not found' could be a viable error
            if e.kind() != std::io::ErrorKind::NotFound {
                error!(
                    "unable to delete snapshot file: {:?}, error: {}",
                    &snapfile, e
                );
            }
        } else {
            info!("deleting snapshot file {:?}", &snapfile);
        }
    }
    info!("using snapshot file:{}", &snapfile.display());

    // read snapshot data from file
    let snapshot = Snapshot::load(&snapfile);
    if let Err(e) = &snapshot {
        Nagios::exit_critical(&format!(
            "unable to load snapshot file: {:?}, error: {}",
            &snapfile, e
        ));
    }

    info!(
        "loaded snapshot file {:?}, data = {:?}",
        &snapfile, &snapshot
    );

    snapshot.unwrap()
}

/// Saves snapshot file into provided path
pub fn save_snapshot(snapshot: &mut Snapshot, snapfile: &PathBuf, retention: u64) {
    debug!("saving snapshot file {}", &snapfile.display());
    if let Err(e) = snapshot.save(&snapfile, retention) {
        Nagios::exit_critical(&format!(
            "unable to save snapshot file: {:?}, error: {}",
            &snapfile, e
        ));
    }
}

// let _config = if options.config_file == PathBuf::from("-") {
//     let mut buffer = String::with_capacity(Cons::DEFAULT_STRING_CAPACITY);
//     let stdin = stdin();
//     let mut handle = stdin.lock();

//     if let Err(e) = handle.read_to_string(&mut buffer) {
//         eprintln!("error reading stdin: {}", e);
//         exit(AppExitCode::STDIN_ERROR as i32);
//     }
//     Config::<LogSource>::from_str(&buffer)
// } else {
//     Config::<LogSource>::from_file(&options.config_file)
// };
// let _config = Config::<LogSource>::from_file(&options.config_file);

// // check for loading errors
// if let Err(error) = _config {
//     eprintln!(
//         "error loading config file: {:?}, error: {}",
//         &options.config_file, error
//     );

//     // break down errors
//     match error.get_ioerror() {
//         Some(_) => exit(AppExitCode::CONFIG_IO_ERROR as i32),
//         None => exit(AppExitCode::CONFIG_ERROR as i32),
//     };
//     //exit(AppExitCode::CONFIG_ERROR as i32);
// }