mod util;

use crate::util::Util;
use rclf::snapshot::Snapshot;

// check exit status
#[test]
fn help() {
    let code = Util::exit_status(&["--help"]);
    assert_eq!(code.unwrap(), 0);
}

// no config file
#[test]
fn conf_not_found() {
    let code = Util::exit_status(&["--config", "foo"]);
    assert_eq!(code.unwrap(), 109);
}

// list options
#[test]
fn list_options() {
    let conf = config_file!(
        "list_options",
        r#"
    # test01
    global:
        path: "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
    "#
    );

    let code = Util::exit_status(&["--config", &conf, "--showopt"]);
    assert_eq!(code.unwrap(), 107);
}

// syntax error
#[test]
fn config_error_01() {
    let conf = config_file!(
        "config_error_01",
        r#"
    # test01
    global:
        path: "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
    "#
    );

    let code = Util::exit_status(&["--config", &conf]);
    assert_eq!(code.unwrap(), 101);
}

// syntax error
#[test]
fn config_error_02() {
    let conf = config_file!(
        "config_error_02",
        r#"---
        searches:
          - logfile: tests/logfiles/small_access.log
            ags:
              - name: http_access_get_or_post
                process: true
                options: "rewind,foo"
                patterns:
                  critical: {
                    regexes: [
                      'GET\s+([/\w]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)',
                    ],
                  }
                  warning: {
                    regexes: [
                      'POST\s+([/\w\.]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)'
                    ],
                    exceptions: [
                      '^\d{2,3}\.'
                    ]
                  }"#
    );

    let code = Util::exit_status(&["--config", &conf]);
    assert_eq!(code.unwrap(), 101);
}

#[test]
fn test_01() {
    let conf = config_file!(
        "test01",
        r#"---
        global:
            snapshot_file: tests/tmp/snapshot.json
        searches:
          - logfile: tests/logfiles/small_access.log
            tags: 
              - name: http_access_get_or_post
                process: true
                options: "rewind"

                script: { 
                  path: "tests/scripts/echovars.py",
                  args: ['arg1', 'arg2', 'arg3']
                } 

                patterns:
                  critical: {
                    regexes: [
                      'GET\s+([/\w]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)',
                    ],
                  }
                  warning: {
                    regexes: [
                      'POST\s+([/\w\.]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)'
                    ],
                    exceptions: [
                      '^\d{2,3}\.'
                    ]
                  }"#
    );

    let output = Util::output(&["--config", &conf, "--clflog", "tests/tmp/clf.log"]).unwrap();
    assert_eq!(output.0, 2);
    assert_eq!(
        Util::to_vec(&output.1)[0],
        "CRITICAL - (errors:443, warnings:28, unknowns:0)"
    );

    // check snapshot file
    let snap = Snapshot::load("tests/tmp/snapshot.json").unwrap();
    let logfile = snap.get_logfile("tests/logfiles/small_access.log");


}

#[test]
fn test_02() {
    let conf = config_file!(
        "test02",
        r#"---
        global:
            snapshot_file: tests/tmp/snapshot.json
        searches:
          - logfile: tests/logfiles/small_access.log.gz
            tags: 
              - name: http_access_get_or_post
                process: true
                options: "rewind"

                script: { 
                  path: "tests/scripts/echovars.py",
                  args: ['arg1', 'arg2', 'arg3']
                } 

                patterns:
                  critical: {
                    regexes: [
                      'GET\s+([/\w]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)',
                    ],
                  }
                  warning: {
                    regexes: [
                      'POST\s+([/\w\.]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)'
                    ],
                    exceptions: [
                      '^\d{2,3}\.'
                    ]
                  }"#
    );

    let output = Util::output(&["--config", &conf, "--clflog", "tests/tmp/clf.log"]).unwrap();
    assert_eq!(output.0, 2);
    assert_eq!(
        Util::to_vec(&output.1)[0],
        "CRITICAL - (errors:443, warnings:28, unknowns:0)"
    );
}
