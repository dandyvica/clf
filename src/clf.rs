// TODO:
// - create a reader for JSON files

// DONE:
// - serialize/deserialize date correctly
// - implement truncate
// - simplify/analyze args.rspath
// - enhance BypassReader display
// - implement prescript/postscript
// - delete unnecessary getters
// - implement fastword option
// - if error calling any callback, don't update counters etc (line_number, offset): done
// - add Tera/Jinja2 templating => add context argument
// - use Config::from_path iso Config::from_file: done FIXME: return code when cmd not working
// - add log rotation facility
// - manage errors when logfile is not found
// - output message: put canon_path iso declared_path
// - add missing variables: CLF_HOSTNAME, CLF_IPADDRESS, CLF_TIMESTAMP, CLF_USER. FIXME: missing CLF_IPADDRESS
// - TODO: implement a unique ID iso pid.
// - implement logfilemissing

use log::{debug, info};
use std::io::ErrorKind;
use std::thread;
use std::time::{Duration, Instant};

#[macro_use]
extern crate log;
extern crate simplelog;

use wait_timeout::ChildExt;

mod configuration;
use configuration::callback::ChildData;

mod logfile;
use logfile::{
    logfileerror::LogFileAccessErrorList,
    lookup::{BypassReader, FullReader, ReaderCallType},
};

mod misc;
use misc::{extension::ReadFs, nagios::Nagios};

mod args;
use args::CliOptions;

mod init;
use init::*;

//use clf::exit_or_unwrap;

/// The main entry point.
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

    // store all logfile access errors
    let mut access_errors = LogFileAccessErrorList::default();

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
    let config = init_config(&options);
    debug!("{:#?}", config);

    // print out config if requested and exit
    if options.check_conf {
        Nagios::exit_ok(&format!("{:#?}", config));
    }

    //---------------------------------------------------------------------------------------------------
    // manage snapshot file: overrides the snapshot file is provided as a command line argument
    //---------------------------------------------------------------------------------------------------
    let (mut snapshot, snapfile) = load_snapshot(&options, &config.global.snapshot_file);

    //---------------------------------------------------------------------------------------------------
    // start prescripts if any
    //---------------------------------------------------------------------------------------------------
    // we'll keep all prescript pid's in order to send them back, if any, to the postscript
    let mut prescript_pids = Vec::new();

    if config.global.prescript.is_some() {
        for prescript in config.global.prescript.as_ref().unwrap() {
            prescript_pids.push(spawn_prescript(prescript, Some(&config.global.global_vars)));
        }
    }

    //---------------------------------------------------------------------------------------------------
    // loop through all searches
    //---------------------------------------------------------------------------------------------------
    for search in &config.searches {
        // log some :qeful info
        info!("==> searching into logfile: {:?}", &search.logfile.path());

        // checks if logfile is accessible. If not, no need to move further, just record last error
        if let Err(e) = search.logfile.path().is_usable() {
            error!(
                "logfile: {:?} is not a file or is not accessible, error: {}",
                &search.logfile.path, e
            );

            // this is an error for this logfile which boils down to a Nagios error
            access_errors.set_error(&search.logfile.path(), e, &search.logfile.logfilemissing);
            continue;
        }

        // create a LogFile struct or get it from snapshot
        let logfile_from_snapshot = {
            let temp = snapshot.logfile_mut(&search.logfile.path(), &search.logfile);
            if let Err(e) = temp {
                error!(
                    "error fetching logfile {} from snapshot: {}",
                    search.logfile.path().display(),
                    e,
                );

                // this is a error for this logfile which boils down to a Nagios unknown error
                access_errors.set_error(&search.logfile.path(), e, &search.logfile.logfilemissing);
                continue;
            }
            temp.unwrap()
        };

        // in case the configuration file changed since the last run and for a logfile, the tags configuration
        // changed, we need to adjust. There're some cases where there could be more tags in the snapshot than
        // in the configuration file. So we need to keep in the snapshot only those in the config file.
        let tag_names = search.tag_names();
        logfile_from_snapshot
            .run_data
            .retain(|k, _| tag_names.contains(&k.as_str()));

        // check if the rotation occured. This means the logfile signature has changed
        trace!(
            "checking if logfile {:?} has changed",
            logfile_from_snapshot.id.canon_path.display()
        );
        let logfile_is_archived = {
            let temp = logfile_from_snapshot.hash_been_rotated();
            if let Err(e) = temp {
                error!(
                    "error on fetching metadata on logfile {}: {}",
                    logfile_from_snapshot.id.canon_path.display(),
                    e
                );
                continue;
            }
            temp.unwrap()
        };

        if logfile_is_archived {
            info!(
                "logfile {} has changed, probably archived and rotated",
                logfile_from_snapshot.id.canon_path.display()
            );

            //let archive_path = LogArchive::default_path(search.logfile.path());
            let archive_path = search.logfile.archive_path();
            trace!("archived logfile = {:?}", &archive_path);

            // clone search and assign archive logfile instead of original logfile
            let mut archived_logfile = logfile_from_snapshot.clone();
            if let Err(e) = archived_logfile
                .id
                .update(&archive_path, archived_logfile.definition.hash_window)
            {
                error!(
                    "error on updating core data on logfile {}: {}",
                    logfile_from_snapshot.id.canon_path.display(),
                    e
                )
            }

            // call adequate reader according to command line
            if reader_type == &ReaderCallType::BypassReaderCall {
                archived_logfile.lookup_tags::<BypassReader>(
                    &config.global,
                    &search.tags,
                    &mut children_list,
                );
            } else if reader_type == &ReaderCallType::FullReaderCall {
                archived_logfile.lookup_tags::<FullReader>(
                    &config.global,
                    &search.tags,
                    &mut children_list,
                );
            }

            // reset run_data into original search because this is a new file
            for tag in &search.tags {
                if !tag.options.savethresholds {
                    logfile_from_snapshot.reset_tag(&tag.name);
                } else {
                    logfile_from_snapshot.reset_tag_offsets(&tag.name);
                    logfile_from_snapshot.copy_counters(&archived_logfile, &tag.name);
                }
            }
        }

        // call adequate reader according to command line
        if reader_type == &ReaderCallType::BypassReaderCall {
            logfile_from_snapshot.lookup_tags::<BypassReader>(
                &config.global,
                &search.tags,
                &mut children_list,
            );
        } else if reader_type == &ReaderCallType::FullReaderCall {
            logfile_from_snapshot.lookup_tags::<FullReader>(
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
    save_snapshot(&mut snapshot, &snapfile, config.global.snapshot_retention);
    trace!("snapshot = {:#?}", &snapshot);

    // teardown
    if !children_list.is_empty() {
        info!(
            "waiting for all processes to finish, nb of children: {}",
            children_list.len()
        );
        wait_children(children_list);
    }

    // optionally call postscript
    if config.global.postscript.is_some() {
        spawn_postscript(&mut config.global.postscript.unwrap(), &prescript_pids);
    }

    info!(
        "end of searches, elapsed: {} seconds",
        now.elapsed().as_secs_f32()
    );

    // now we can prepare the global hit counters to exit the relevant Nagios code
    let exit_code = snapshot.exit_message(&access_errors);
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
    for (i, started_child) in children_list
        .iter()
        .filter(|x| x.child.is_some())
        .enumerate()
    {
        // get a mutable reference
        let mut child = started_child.child.as_ref().unwrap().borrow_mut();

        // save pid & path
        let pid = child.id();
        let path = &started_child.path;

        debug!(
            "managing end of process #{}, pid:{}, path:{}",
            i,
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
