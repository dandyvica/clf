// TODO:
// - create a reader for JSON files
// - implement logfilemissing
// - add missing variables: CLF_HOSTNAME, CLF_IPADDRESS, CLF_TIMESTAMP, CLF_USER
// - implement a unique ID iso pid. FIXME: check exit message from snapshot
// - manage errors when logfile is not found

// DONE:
// - serialize/deserialize date correctly
// - implement truncate
// - simplify/analyze args.rspath
// - enhance BypassReader display
// - implement prescript/postscript
// - delete unnecessary getters
// - implement fastword option
// - FIXME: if error calling any callback, don't update counters etc (line_number, offset)
// - add Tera/Jinja2 templating => add context argument
// - use Config::from_path iso Config::from_file: done FIXME: return code when cmd not working
// - add log rotation facility: FIXME: test it !


use log::{debug, info};
use std::io::ErrorKind;
use std::thread;
use std::time::{Duration, Instant};

#[macro_use]
extern crate log;
extern crate simplelog;

use wait_timeout::ChildExt;

mod config;
use config::callback::ChildData;

mod logfile;
use logfile::lookup::{BypassReader, FullReader, ReaderCallType};

mod misc;
use misc::{extension::ReadFs, nagios::Nagios};

mod args;
use args::CliOptions;

mod init;
use init::*;

//use clf::exit_or_unwrap;

fn main() {
    //---------------------------------------------------------------------------------------------------
    // set up variables
    //---------------------------------------------------------------------------------------------------

    // tick time
    let now = Instant::now();

    // create a vector of thread handles for keeping track of what we've created and
    // wait for them to finish
    let mut children_list: Vec<ChildData> = Vec::new();

    // manage arguments from command line
    let options = CliOptions::options();

    //---------------------------------------------------------------------------------------------------
    // initialize logger
    //---------------------------------------------------------------------------------------------------
    init_log(&options);

    //---------------------------------------------------------------------------------------------------
    // which kind or reader do we want ?
    //---------------------------------------------------------------------------------------------------
    let reader_type = &options.reader_type;

    //---------------------------------------------------------------------------------------------------
    // load configuration file as specified from the command line
    //---------------------------------------------------------------------------------------------------
    let mut config = init_config(&options);
    debug!("{:#?}", config);

    // print out config if requested and exit
    if options.check_conf {
        Nagios::exit_ok(&format!("{:#?}", config));
    }

    //---------------------------------------------------------------------------------------------------
    // manage snapshot file: overrides the snapshot file is provided as a command line argument
    //---------------------------------------------------------------------------------------------------
    if let Some(snap_file) = &options.snapshot_file {
        config.global.snapshot_file = snap_file.to_path_buf();
    }

    let mut snapshot = load_snapshot(&options);

    //---------------------------------------------------------------------------------------------------
    // start the prescript if any
    //---------------------------------------------------------------------------------------------------
    let mut prescript_pid = 0;
    if config.global.prescript.is_some() {
        // execute script
        let prescript = &config.global.prescript.as_ref().unwrap();
        let result = prescript.spawn();

        // check rc
        if let Err(e) = &result {
            error!("error: {} spawning command: {:?}", e, prescript.command);
            Nagios::exit_critical(&format!(
                "error: {} spawning command: {:?}",
                e, prescript.command
            ));
        }

        // now it's safe to unwrap to get pid
        prescript_pid = result.unwrap();

        info!(
            "prescript command successfully executed, pid={}",
            prescript_pid
        );
    }

    //---------------------------------------------------------------------------------------------------
    // loop through all searches
    //---------------------------------------------------------------------------------------------------
    for search in &config.searches {
        // log some useful info
        info!("==> searching into logfile: {:?}", &search.logfile.path);

        // checks if logfile is accessible. If not, no need to move further, just record last error
        // if let Err(e) = search.logfile.path().is_usable() {
        //     error!(
        //         "logfile: {:?} is not a file or is not accessible, error: {}",
        //         &search.logfile.path, e
        //     );

        //     // this is a error for this logfile which boils down to a Nagios unknown error
        //     //hit_counter.set_error(e);

        //     continue;
        // }

        // create a LogFile struct or get it from snapshot
        let snapshot_logfile = {
            let temp = snapshot.logfile_mut(&search.logfile.path(), &search.logfile);
            if let Err(e) = temp {
                error!(
                    "error fetching logfile {} from snapshot: {}",
                    search.logfile.path().display(),
                    e,
                );

                // this is a error for this logfile which boils down to a Nagios unknown error
                //hit_counter.set_error(e);

                continue;
            }
            temp.unwrap()
        };

        // check if the rotation occured. This means the logfile signature has changed
        let logfile_is_archived = {
            let temp = snapshot_logfile.has_changed();
            if let Err(e) = temp {
                error!(
                    "error on fetching metadata on logfile {}: {}",
                    snapshot_logfile.path.display(),
                    e
                );

                // this is a error for this logfile which boils down to a Nagios unknown error
                //snapshot_logfile.set_error(e);

                continue;
            }
            temp.unwrap()
        };

        if logfile_is_archived {
            info!("logfile has changed, probably archived and rotated");

            // get archive file name
            // first, check if an archive tag has been defined in the YAML config for this search
            if search.logfile.archive.is_none() {
                error!("logfile {} has been moved or archived but no archive settings defined in the configuration file", snapshot_logfile.path.display());
                break;
            }

            let archive_path = search.logfile.archive.as_ref().unwrap();

            // clone search and assign archive logfile instead of original logfile
            let mut archive_snapshot_logfile = snapshot_logfile.clone();
            archive_snapshot_logfile.update(&archive_path);

            // call adequate reader according to command line
            if reader_type == &ReaderCallType::BypassReaderCall {
                snapshot_logfile.lookup_tags::<BypassReader>(
                    &config.global,
                    &search.tags,
                    &mut children_list,
                );
            } else if reader_type == &ReaderCallType::FullReaderCall {
                snapshot_logfile.lookup_tags::<FullReader>(
                    &config.global,
                    &search.tags,
                    &mut children_list,
                );
            }

            // reset run_data into original search because this is a new file
            snapshot_logfile.run_data.clear();
        }

        // call adequate reader according to command line
        if reader_type == &ReaderCallType::BypassReaderCall {
            snapshot_logfile.lookup_tags::<BypassReader>(
                &config.global,
                &search.tags,
                &mut children_list,
            );
        } else if reader_type == &ReaderCallType::FullReaderCall {
            snapshot_logfile.lookup_tags::<FullReader>(
                &config.global,
                &search.tags,
                &mut children_list,
            );
        }
    }

    // just exit if the '--no-callback' option was used
    if reader_type == &ReaderCallType::BypassReaderCall {
        Nagios::exit_ok("read complete");
    }

    // save snapshot and optionally delete old entries
    save_snapshot(
        &mut snapshot,
        &config.global.snapshot_file,
        config.global.snapshot_retention,
    );

    // teardown
    info!(
        "waiting for all processes to finish, nb of children: {}",
        children_list.len()
    );
    wait_children(children_list);

    // optionally call postscript
    if config.global.postcript.is_some() {
        // add the pid to the end of arguments
        let postcript = &mut config.global.postcript.as_mut().unwrap();
        postcript.command.push(prescript_pid.to_string());

        // run script
        let result = postcript.spawn();

        // check rc
        if let Err(e) = &result {
            error!("error: {} spawning command: {:?}", e, postcript.command);
        } else {
            info!(
                "postcript command successfully executed, pid={}",
                prescript_pid
            )
        }
    }

    info!(
        "end of searches, elapsed: {} seconds",
        now.elapsed().as_secs_f32()
    );

    // now we can prepare the global hit counters to exit the relevant Nagios code
    let exit_code = snapshot.exit_message();
    Nagios::exit_with(exit_code);
}

/// Manage end of all started processes from clf.
fn wait_children(children_list: Vec<ChildData>) {
    // just wait a little for all commands to finish. Otherwise, the last process will not be considered to be finished.
    if !children_list.is_empty() {
        let wait_timeout = std::time::Duration::from_millis(1000);
        thread::sleep(wait_timeout);
    }

    // as child can be None in case of Tcp or Domain socket, need to get rid of these
    for started_child in children_list.iter().filter(|x| x.child.is_some()) {
        // get a mutable reference
        let mut child = started_child.child.as_ref().unwrap().borrow_mut();

        // save pid & path
        let pid = child.id();
        let path = &started_child.path;

        debug!(
            "managing end of process, pid:{}, path:{}",
            pid,
            path.display()
        );

        // use try_wait() to check if command has exited
        match child.try_wait() {
            // child has already exited. So check output status code if any
            Ok(Some(status)) => debug!(
                "command with path: {}, pid: {} exited with: {}",
                path.display(),
                pid,
                status
            ),

            // child has not exited. Spawn a new thread to wait at most the timeout defined
            Ok(None) => {
                debug!("command has not exited yet, try to wait a little!");

                // now if timeout has not yet occured, start a new thread to wait and kill process ??
                let elapsed = started_child.start_time.unwrap().elapsed().as_secs();

                // if timeout occured, try to kill anyway ;-)
                if elapsed > started_child.timeout {
                    match child.kill() {
                        Ok(_) => info!("process {} killed", child.id()),
                        Err(e) => {
                            if e.kind() == ErrorKind::InvalidInput {
                                info!("process {} already killed", child.id());
                            } else {
                                info!(
                                    "error:{} trying to kill process pid:{}, path: {}",
                                    e,
                                    pid,
                                    path.display()
                                );
                            }
                        }
                    }
                } else {
                    // we'll wait at least the remaining seconds
                    let secs_to_wait = Duration::from_secs(started_child.timeout - elapsed);

                    let _status_code = match child.wait_timeout(secs_to_wait).unwrap() {
                        Some(status) => status.code(),
                        None => {
                            // child hasn't exited yet
                            child.kill().unwrap();
                            child.wait().unwrap().code()
                        }
                    };
                }
            }

            // unlikely error
            Err(e) => eprintln!("error attempting to try_wait: {} for pid:{}", e, pid),
        };
    }
}
