// TODO:
// - create a reader for JSON files
// - serialize/deserialize date correctly: done
// - add Tera/Jinja2 templating => add context argument
// - delete tag_name is snapshot: done
// - add log rotation facility
// - simplify/analyze args.rs: done
// - enhance BypassReader display: done
// - use Config::from_path iso Config::from_file: done

use log::{debug, info};
use std::io::ErrorKind;
use std::process::id;
use std::thread;
use std::time::{Duration, Instant};

#[macro_use]
extern crate log;
extern crate simplelog;

use wait_timeout::ChildExt;

mod config;
use config::callback::ChildData;

mod logfile;
use logfile::{
    logfile::{LogFile, Wrapper},
    lookup::{BypassReader, FullReader, ReaderCallType},
};

mod misc;
use misc::{
    nagios::{LogfileHitCounter, Nagios, NagiosError, NagiosVersion},
    util::Usable,
};

mod args;
use args::CliOptions;

mod testing;

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

    // and this for each invididual file
    let mut logfile_counter = LogfileHitCounter::default();

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
        config.set_snapshot_file(&snap_file);
    }

    let mut snapshot = load_snapshot(&options);

    //---------------------------------------------------------------------------------------------------
    // loop through all searches
    //---------------------------------------------------------------------------------------------------
    for search in &config.searches {
        // log some useful info
        info!("------------ searching into logfile: {}", search.logfile);

        // get counter corresponding to the logfile
        let mut hit_counter = &mut logfile_counter.or_default(&search.logfile());

        // checks if logfile is accessible. If not, no need to move further, just record last error
        if let Err(e) = search.logfile().is_usable() {
            error!(
                "logfile: {} is not a file or is not accessible, error: {}",
                search.logfile, e
            );

            // this is a error for this logfile which boils down to a Nagios unknown error
            hit_counter.set_error(e);

            continue;
        }

        // create a LogFile struct or get it from snapshot
        let snapshot_logfile = {
            let temp = snapshot.logfile_mut(&search.logfile());
            if let Err(e) = temp {
                error!(
                    "error fetching logfile {} from snapshot: {}",
                    search.logfile().display(),
                    e,
                );

                // this is a error for this logfile which boils down to a Nagios unknown error
                hit_counter.set_error(e);

                continue;
            }
            temp.unwrap()
        };

        // check if the rotation occured. This means the logfile signature has changed
        let snapshot_has_changed = {
            let temp = snapshot_logfile.has_changed();
            if let Err(e) = temp {
                error!(
                    "error on fetching metadata on logfile {}: {}",
                    snapshot_logfile.path().display(),
                    e
                );

                // this is a error for this logfile which boils down to a Nagios unknown error
                hit_counter.set_error(e);

                continue;
            }
            temp.unwrap()
        };

        if snapshot_has_changed {
            info!("logfile has changed, probably archived and rotated");

            // first, check if an archive tag has been defined in the YAML config for this search
            if search.archive.is_none() {
                error!("logfile {} has been moved or archived but no archive settings defined in the configuration file", snapshot_logfile.path.display());
                continue;
            }

            //     // get archived log file name. Now it's safe to unwrap
            //     let archived_path = search
            //         .archive
            //         .as_ref()
            //         .unwrap()
            //         .archived_path(&snapshot_logfile.path);

            //     if archived_path.is_none() {
            //         error!(
            //             "can't determine archived logfile for {}",
            //             snapshot_logfile.path.display()
            //         );
            //         continue;
            //     }

            //     // create a new instance of the logfile with the archived file
            //     let mut _archived_logfile = {
            //         let temp = LogFile::from_path(&archived_path.unwrap());
            //         if let Err(e) = temp {
            //             error!(
            //                 "error on creating logfile for path {}: {}",
            //                 snapshot_logfile.path().display(),
            //                 e
            //             );

            //             // this is a error for this logfile which boils down to a Nagios unknown error
            //             hit_counter.set_error(e);

            //             continue;
            //         }
            //         temp.unwrap()
            //     };

            //     // duplicate rundata from the original logfile
            //     _archived_logfile.set_rundata(&snapshot_logfile.rundata());

            //     // finally, the archive logfile is ready to be processed
            //     archived_logfile = Some(_archived_logfile);
            // }

            // // build a new queue to manage archve & brand new file
            //let mut queue = LogQueue::new(snapshot_logfile);
            // if archived_logfile.is_some() {
            //     queue.set_rotated(archived_logfile.as_mut())
        }

        if reader_type == &ReaderCallType::BypassReaderCall {
            snapshot_logfile.lookup_tags::<BypassReader>(
                config.global(),
                &search.tags,
                &mut hit_counter,
                &reader_type,
                &mut children_list,
            );
        } else if reader_type == &ReaderCallType::FullReaderCall {
            snapshot_logfile.lookup_tags::<FullReader>(
                config.global(),
                &search.tags,
                &mut hit_counter,
                &reader_type,
                &mut children_list,
            );
        }
    }

    // just exit if the --no-call option was used
    if reader_type == &ReaderCallType::BypassReaderCall {
        Nagios::exit_ok("read complete");
    }

    // save snapshot and optionally delete old entries
    save_snapshot(
        &mut snapshot,
        config.snapshot_file(),
        config.snapshot_retention(),
    );

    // teardown
    info!(
        "waiting for all processes to finish, nb of children: {}",
        children_list.len()
    );
    wait_children(children_list);

    info!(
        "end of searches, elapsed: {} seconds",
        now.elapsed().as_secs_f32()
    );

    // display output to comply to Nagios plug-in convention
    debug!("logfile exit counters: {:?}", logfile_counter);

    let exit_code = nagios_output(&logfile_counter, &options.nagios_version);
    info!("exiting process pid:{}, exit code:{:?}", id(), exit_code);
    Nagios::exit_with(exit_code);
}

/// Lookup data from tags
// fn lookup(tags: &[Tag], logile: &LogFile, global: &GlobalOptions, vars: &Variables, hit_counter: &HitCounter) {
//         // for each tag, search inside logfile for those we need to process (having process tag == true)
//         for tag in search.tags.iter().filter(|t| t.process()) {
//             debug!("searching for tag: {}", &tag.name());

//             // wraps all structures into a helper struct
//             let mut wrapper = Wrapper::new(config.global(), &tag, &mut vars, &mut hit_counter);

//             // now we can search for the pattern and save the child handle if a script was called
//             match snapshot_logfile.lookup(&mut wrapper) {
//                 // script might be started, giving back a `Child` structure with process features like pid etc
//                 Ok(mut children) => {
//                     // merge list of children
//                     if children.len() != 0 {
//                         children_list.append(&mut children);
//                     }
//                 }

//                 // otherwise, an error when opening (most likely) the file and then report an error on counters
//                 Err(e) => {
//                     error!(
//                         "error: {} when searching logfile: {} for tag: {}",
//                         e,
//                         search.logfile.display(),
//                         &tag.name()
//                     );

//                     // set error for this logfile
//                     hit_counter.set_error(e);
//                 }
//             }
//         }
// }

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

/// Manage Nagios output, depending on the NRPE version.
fn nagios_output(
    logfile_counter: &LogfileHitCounter,
    nagios_version: &NagiosVersion,
) -> NagiosError {
    // calculate global hits
    let global = logfile_counter.global();
    println!("{}", global);

    // plugin output depends on the Nagios version
    match nagios_version {
        NagiosVersion::Nrpe3 => {
            println!("{}", logfile_counter);
        }

        NagiosVersion::Nrpe2 => {}
    };

    // return Nagios exit status coming from global hits
    NagiosError::from(&global)
}
