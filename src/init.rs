//! All preliminary steps to prepare reading files
use std::fs::OpenOptions;
use std::path::PathBuf;

use simplelog::*;

use crate::args::CliOptions;
use crate::config::config::Config;
use crate::logfile::snapshot::Snapshot;
use crate::misc::nagios::Nagios;

/// Create a new config struct
pub fn init_config(options: &CliOptions) -> Config {
    #[cfg(feature = "tera")]
    let _config = Config::from_path(&options.config_file, options.tera_context.as_deref());

    #[cfg(not(feature = "tera"))]
    let _config = Config::from_path(&options.config_file);

    // check for loading errors
    if let Err(ref e) = _config {
        Nagios::exit_critical(&format!(
            "error loading config file: {:?}, error: {}",
            &options.config_file, e
        ));
    }

    let config = Config::from(_config.unwrap());
    config
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

/// Load the snapshot file: if option "-p" is present, use it, or use the config tag or build a new name from config file
pub fn load_snapshot(options: &CliOptions, config_snap: &Option<PathBuf>) -> (Snapshot, PathBuf) {
    // if option "-p" is present, use it, or use the config tag or build a new name from config file
    let snapfile = if options.snapshot_file.is_some() {
        options.snapshot_file.as_ref().unwrap().clone()
    } else if config_snap.is_some() {
        config_snap.as_ref().unwrap().clone()
    } else {
        Snapshot::build_name(&options.config_file)
    };

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

    (snapshot.unwrap(), snapfile)
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
