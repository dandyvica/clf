use clap::{App, Arg};
use std::path::PathBuf;

mod testcase;
use testcase::*;

fn main() {
    // manage cli arguments
    let matches = App::new("Log files reader")
        .version("0.1")
        .author("Alain Viguier dandyvica@gmail.com")
        .about(r#"Run intergation tests with clf"#)
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .required(false)
                .long_about("Debug or release")
                .possible_values(&["debug", "release"])
                .takes_value(true),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .required(false)
                .long_about("If set, show clf standard output when running test cases")
                .takes_value(false),
        )
        .arg(
            Arg::new("clf")
                .short('c')
                .long("clf")
                .required(false)
                .long_about("Path of the clf executable. Defaults to ./target/debug/clf or ./target/release/clf")
                .takes_value(true),
        )
        .arg(
            Arg::new("testcase")
                .short('t')
                .long("testcase")
                .required(false)
                .long_about("A list of testcases to execute. If not specified, all testcases are run")
                .multiple(true)
                .takes_value(true),
        )
        .get_matches();

    let mut opts = Options::default();

    opts.mode = matches.value_of_t("mode").unwrap_or(Target::debug);
    opts.verbose = matches.is_present("verbose");
    opts.clf = matches
        .value_of_t("clf")
        .unwrap_or(opts.mode.path().to_string());

    let mut testcases: Vec<&str> = Vec::new();
    if matches.is_present("testcase") {
        testcases = matches.values_of("testcase").unwrap().collect();
    }

    //println!("options={:?}", opts);
    let mut nb_testcases: u16 = 1;

    //------------------------------------------------------------------------------------------------
    // prepare run by creating necessary directories if necessary
    //------------------------------------------------------------------------------------------------
    TestScenario::prepare();

    // update environnement to include DLL if Windows
    #[cfg(target_family = "windows")]
    {
        let path = std::env::var("PATH").expect("unable to fetch %PATH%");
        let new_path = format!(r"{};.\src\windows", path);
        std::env::set_var("PATH", new_path);
    }

    //------------------------------------------------------------------------------------------------
    // command line flags
    //------------------------------------------------------------------------------------------------

    // call help
    if testcases.is_empty() || testcases.contains(&"help") {
        let tc = TestCase::new("help", &mut nb_testcases);
        let rc = tc.exec(&opts, &["--help"]);

        assert_eq!(rc, 0);
    }

    // missing argument
    if testcases.is_empty() || testcases.contains(&"missing_argument") {
        let tc = TestCase::new("missing_argument", &mut nb_testcases);
        let rc = tc.exec(&opts, &["--syntax-check"]);

        assert_eq!(rc, 2);
    }

    // good YAML syntax
    if testcases.is_empty() || testcases.contains(&"good_syntax") {
        let tc = TestCase::new("good_syntax", &mut nb_testcases);
        let rc = tc.exec(
            &opts,
            &[
                "--config",
                "./tests/integration/config/generated.yml",
                "--syntax-check",
            ],
        );

        assert_eq!(rc, 0);
    }

    // bad YAML syntax
    if testcases.is_empty() || testcases.contains(&"bad_syntax") {
        let tc = TestCase::new("bad_syntax", &mut nb_testcases);
        let rc = tc.exec(
            &opts,
            &[
                "--config",
                "./tests/intergration/config/bad_syntax.yml",
                "--syntax-check",
            ],
        );

        assert_eq!(rc, 2);
    }

    // show options
    if testcases.is_empty() || testcases.contains(&"show_options") {
        let tc = TestCase::new("show_options", &mut nb_testcases);
        let rc = tc.exec(
            &opts,
            &[
                "--config",
                "./tests/integration/config/generated.yml",
                "--syntax-check",
                "--show-options",
            ],
        );

        assert_eq!(rc, 0);
    }

    //------------------------------------------------------------------------------------------------
    // genuine search with fastforward
    //------------------------------------------------------------------------------------------------
    // rewind
    if testcases.is_empty() || testcases.contains(&"rewind") {
        let mut tc = TestCase::new("rewind", &mut nb_testcases);
        Config::default()
            .set_tag("options", "rewind")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d", "-r"]);

        jassert!(tc, "compression", "uncompressed");
        jassert!(tc, "extension", "log");
        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);

        // run another time
        let rc = tc.run(&opts, &[]);

        jassert!(tc, "compression", "uncompressed");
        jassert!(tc, "extension", "log");
        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
    }

    // fastforward
    if testcases.is_empty() || testcases.contains(&"fastforward") {
        let mut tc = TestCase::new("fastforward", &mut nb_testcases);
        Config::default()
            .set_tag("options", "fastforward")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "compression", "uncompressed");
        jassert!(tc, "extension", "log");
        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "0");
        jassert!(tc, "warning_count", "0");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 0);

        // run another time
        tc.grow();
        let rc = tc.run(&opts, &[]);

        jassert!(tc, "compression", "uncompressed");
        jassert!(tc, "extension", "log");
        jassert!(tc, "last_offset", "40200");
        jassert!(tc, "last_line", "402");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
    }

    // fastforward gzipped
    if testcases.is_empty() || testcases.contains(&"fastforward_gzipped") {
        let mut tc = TestCase::new("fastforward_gzipped", &mut nb_testcases);
        // gzip log
        tc.gzip();

        Config::default()
            .set_tag("options", "fastforward")
            .set_tag("path", "./tests/integration/tmp/fastforward_gzipped.log.gz")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "compression", "gzip");
        jassert!(tc, "extension", "gz");
        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "0");
        jassert!(tc, "warning_count", "0");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 0);
        jassert!(rc, "OK");
    }

    // logfile missing
    if testcases.is_empty() || testcases.contains(&"logfilemissing") {
        let mut tc = TestCase::new("logfilemissing", &mut nb_testcases);

        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", "./tmp/my_foo_file")
            .set_tag("logfilemissing", "critical")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", "./tmp/my_foo_file")
            .set_tag("logfilemissing", "warning")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);
        assert_eq!(rc.0, 1);
        jassert!(rc, "warning");

        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", "./tmp/my_foo_file")
            .set_tag("logfilemissing", "unknown")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);
        assert_eq!(rc.0, 3);
        jassert!(rc, "UNKNOWN");
    }

    //------------------------------------------------------------------------------------------------
    // dummy search
    //------------------------------------------------------------------------------------------------
    // ascii
    if testcases.is_empty() || testcases.contains(&"dummy") {
        let mut tc = TestCase::new("dummy", &mut nb_testcases);
        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
    }

    // utf8
    if testcases.is_empty() || testcases.contains(&"utf8") {
        let mut tc = TestCase::new("utf8", &mut nb_testcases);
        tc.create_log_utf8();

        Config::from_file("./tests/integration/config/utf8.yml")
            .set_tag("options", "protocol")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "26128");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "100");
        jassert!(tc, "warning_count", "0");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
    }

    // tera
    if testcases.is_empty() || testcases.contains(&"tera") {
        let mut tc = TestCase::new("tera", &mut nb_testcases);
        Config::from_file("./tests/integration/config/tera.yml")
            .set_tag("options", "protocol")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let context = r#"{"path":"./tests/integration/tmp/generated.log", "format":"plain"}"#;
        let rc = tc.run(&opts, &["-d", "-x", context]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
    }

    // check retention deletion
    if testcases.is_empty() || testcases.contains(&"retention") {
        let mut tc = TestCase::new("retention", &mut nb_testcases);

        // run once
        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let _ = tc.run(&opts, &["-d"]);

        // wait a little before calling a second time to test retention
        let timeout = std::time::Duration::from_millis(2000);
        std::thread::sleep(timeout);

        // now change logfile: reuse previous one
        let new_file = "./tests/integration/tmp/tera.log";
        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", &new_file)
            .save_as(&tc.config_file);
        let _ = tc.run(&opts, &[]);

        // read whole snapshot file
        let snap =
            std::fs::read_to_string(&tc.snap_file).expect("unable to read JSON for retention");

        // should contain a single logfile
        assert!(snap.contains("./tests/integration/tmp/tera.log"));
        assert!(!snap.contains("./tests/integration/tmp/retention.log"));

        // jassert!(tc, "last_offset", "20100");
        // jassert!(tc, "last_line", "201");
        // jassert!(tc, "critical_count", "99");
        // jassert!(tc, "warning_count", "98");
        // jassert!(tc, "ok_count", "0");
        // jassert!(tc, "exec_count", "0");
        // assert_eq!(rc.0, 2);
    }

    // exclude
    if testcases.is_empty() || testcases.contains(&"exclude") {
        let mut tc = TestCase::new("exclude", &mut nb_testcases);

        Config::from_file("./tests/integration/config/exclude.yml")
            .set_tag("options", "protocol")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "0");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 1);
    }

    // truncate
    if testcases.is_empty() || testcases.contains(&"truncate") {
        let mut tc = TestCase::new("truncate", &mut nb_testcases);
        Config::default()
            .set_tag("options", "truncate=10")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "0");
        jassert!(tc, "warning_count", "0");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 0);
    }

    //------------------------------------------------------------------------------------------------
    // test snapshot file creation options
    //------------------------------------------------------------------------------------------------
    if testcases.is_empty() || testcases.contains(&"snapshot_creation") {
        let mut tc = TestCase::new("snapshot_creation", &mut nb_testcases);
        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);

        // run with command line argument
        let _ = tc.run(&opts, &["-d"]);
        assert!(PathBuf::from(&tc.snap_file).is_file());

        // run without specifying a snapshot on the command line
        let _ = tc.exec(&opts, &["--config", &tc.config_file]);
        assert!(PathBuf::from("./tests/integration/tmp/snapshot_foo.json").is_file());

        // run without specifying a snapshot on the command line but tag snapshot_file is a directory
        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", &tc.logfile)
            .set_tag("snapshot_file", "./tests/integration/tmp")
            .save_as(&tc.config_file);

        let _ = tc.exec(&opts, &["--config", &tc.config_file]);
        assert!(PathBuf::from("./tests/integration/tmp/snapshot_creation.json").is_file());
    }

    //------------------------------------------------------------------------------------------------
    // list files Unix & Windows
    //------------------------------------------------------------------------------------------------
    if testcases.is_empty() || testcases.contains(&"list_files") {
        let mut tc = TestCase::new("list_files", &mut nb_testcases);
        tc.multiple_logs();

        #[cfg(target_family = "unix")]
        Config::from_file("./tests/integration/config/list_files.yml")
            .set_tag("options", "protocol")
            .set_tag(
                "list",
                r#"["find", "./tests/integration/tmp", "-type", "f", "-name", "list_files.log.*"]"#,
            )
            .save_as(&tc.config_file);
        #[cfg(target_os = "windows")]
        Config::from_file("./tests/integration/config/list_files.yml")
            .set_tag("options", "protocol")
            .set_tag(
                "list",
                r#"['cmd.exe', '/c', 'dir /B /S .\tests\integration\tmp\list_files.log.*']"#,
            )
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d", "-r"]);

        assert_eq!(rc.0, 2);
        jassert!(rc, "list_files.log");
    }

    //------------------------------------------------------------------------------------------------
    // exit_msg
    //------------------------------------------------------------------------------------------------
    if testcases.is_empty() || testcases.contains(&"exit_msg") {
        let mut tc = TestCase::new("exit_msg", &mut nb_testcases);
        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let _ = tc.run(&opts, &["-d"]);

        // run a second time by changing the logfile
        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", "./tests/integration/tmp/exit_msg_2.log")
            .set_tag("snapshot_retention", "3600")
            .save_as(&tc.config_file);
        tc.create_log(Some("./tests/integration/tmp/exit_msg_2.log"), false);
        let rc = tc.run(&opts, &[]);

        assert_eq!(rc.0, 2);
        assert!(rc
            .1
            .contains("CRITICAL: (errors:99, warnings:98, unknowns:0)"));
    }

    //------------------------------------------------------------------------------------------------
    // extra variables
    //------------------------------------------------------------------------------------------------
    #[cfg(target_family = "unix")]
    if testcases.is_empty() || testcases.contains(&"extra_vars") {
        let mut tc = TestCase::new("extra_vars", &mut nb_testcases);
        Config::default()
            .set_tag("options", "runcallback,stopat=5")
            .set_tag("path", &tc.logfile)
            .replace_tag(
                "address",
                "script",
                "./tests/integration/callbacks/echovars.py",
            )
            .set_tag("args", "['./tests/integration/tmp/extra_vars.txt', 'arg2']")
            .save_as(&tc.config_file);
        let _ = tc.run(
            &opts,
            &[
                "-d",
                "--var",
                "CLF_EXTRA_VAR1:value1",
                "CLF_EXTRA_VAR2:value2",
            ],
        );

        // check reuslting file created from running script
        let data: String = std::fs::read_to_string(&tc.tmpfile)
            .expect(&format!("unable to open file {}", &tc.tmpfile));
        assert!(data.contains(&"CLF_EXTRA_VAR1"));
        assert!(data.contains(&"CLF_EXTRA_VAR2"));
        assert!(data.contains(&"value1"));
        assert!(data.contains(&"value2"));
    }

    //------------------------------------------------------------------------------------------------
    // ok pattern
    //------------------------------------------------------------------------------------------------
    // ok pattern but runifok = false
    if testcases.is_empty() || testcases.contains(&"ok_pattern") {
        let mut tc = TestCase::new("ok_pattern", &mut nb_testcases);
        Config::from_file("./tests/integration/config/ok_pattern.yml")
            .set_tag("options", "protocol")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "74");
        jassert!(tc, "warning_count", "73");
        jassert!(tc, "ok_count", "1");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
    }

    // ok pattern but runifok = true
    if testcases.is_empty() || testcases.contains(&"runifok") {
        let mut tc = TestCase::new("runifok", &mut nb_testcases);
        #[cfg(target_family = "unix")]
        Config::from_file("./tests/integration/config/ok_pattern.yml")
            .set_tag("options", "runcallback,runifok")
            .set_tag("path", &tc.logfile)
            .replace_tag(
                "address",
                "script",
                "./tests/integration/callbacks/echovars.py",
            )
            .set_tag("args", "['./tests/integration/tmp/runifok.txt', 'arg2']")
            .save_as(&tc.config_file);
        #[cfg(target_family = "windows")]
        Config::from_file("./tests/integration/config/ok_pattern.yml")
            .set_tag("options", "runcallback,runifok")
            .set_tag("path", &tc.logfile)
            .replace_tag("address", "script", "python.exe")
            .set_tag(
                "args",
                r"['.\tests\integration\scripts\echovars.py', '.\tests\integration\tmp\runifok.txt']",
            )
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "74");
        jassert!(tc, "warning_count", "73");
        jassert!(tc, "ok_count", "1");
        jassert!(tc, "exec_count", "198");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // check reuslting file created from running script
        let data: String = std::fs::read_to_string(&tc.tmpfile)
            .expect(&format!("unable to open file {}", &tc.tmpfile));
        assert_eq!(data.chars().filter(|c| *c == '\n').count(), 198);
    }

    //------------------------------------------------------------------------------------------------
    // thresholds
    //------------------------------------------------------------------------------------------------
    // criticalthreshold
    if testcases.is_empty() || testcases.contains(&"thresholds") {
        let mut tc = TestCase::new("thresholds", &mut nb_testcases);
        Config::default()
            .set_tag("options", "criticalthreshold=50,warningthreshold=60")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "49");
        jassert!(tc, "warning_count", "38");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");
    }

    // warningthreshold
    if testcases.is_empty() || testcases.contains(&"huge_thresholds") {
        let mut tc = TestCase::new("huge_thresholds", &mut nb_testcases);
        Config::default()
            .set_tag("options", "criticalthreshold=1500,warningthreshold=1500")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "0");
        jassert!(tc, "warning_count", "0");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 0);
        jassert!(rc, "OK");
    }

    //------------------------------------------------------------------------------------------------
    // run scripts
    //------------------------------------------------------------------------------------------------
    // run a script
    if testcases.is_empty() || testcases.contains(&"start_script") {
        let mut tc = TestCase::new("start_script", &mut nb_testcases);
        #[cfg(target_family = "unix")]
        Config::default()
            .set_tag("options", "runcallback")
            .set_tag("path", &tc.logfile)
            .replace_tag(
                "address",
                "script",
                "./tests/integration/callbacks/echovars.py",
            )
            .set_tag(
                "args",
                "['./tests/integration/tmp/start_script.txt', 'arg2']",
            )
            .save_as(&tc.config_file);
        #[cfg(target_family = "windows")]
        Config::default()
            .set_tag("options", "runcallback")
            .set_tag("path", &tc.logfile)
            .replace_tag("address", "script", "python.exe")
            .set_tag(
                "args",
                r"['.\tests\integration\scripts\echovars.py', '.\tests\integration\tmp\start_script.txt']",
            )
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "197");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // check reuslting file created from running script
        let data: String = std::fs::read_to_string(&tc.tmpfile)
            .expect(&format!("unable to open file {}", &tc.tmpfile));
        assert_eq!(data.chars().filter(|c| *c == '\n').count(), 197);
    }

    // run a script with a threshold
    if testcases.is_empty() || testcases.contains(&"script_threshold") {
        let mut tc = TestCase::new("script_threshold", &mut nb_testcases);
        #[cfg(target_family = "unix")]
        Config::default()
            .set_tag(
                "options",
                "runcallback,criticalthreshold=50,warningthreshold=60",
            )
            .set_tag("path", &tc.logfile)
            .replace_tag(
                "address",
                "script",
                "./tests/integration/callbacks/echovars.py",
            )
            .set_tag(
                "args",
                "['./tests/integration/tmp/script_threshold.txt', 'arg2']",
            )
            .save_as(&tc.config_file);
        #[cfg(target_family = "windows")]
        Config::default()
            .set_tag(
                "options",
                "runcallback,criticalthreshold=50,warningthreshold=60",
            )
            .set_tag("path", &tc.logfile)
            .replace_tag("address", "script", "python.exe")
            .set_tag("args", r"['.\tests\integration\scripts\echovars.py', '.\tests\integration\tmp\script_threshold.txt']")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "49");
        jassert!(tc, "warning_count", "38");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "87");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");
    }

    // stop at, no savethresholds
    if testcases.is_empty() || testcases.contains(&"stopat") {
        let mut tc = TestCase::new("stopat", &mut nb_testcases);
        Config::default()
            .set_tag("options", "stopat=70")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "6900");
        jassert!(tc, "last_line", "69");
        jassert!(tc, "critical_count", "34");
        jassert!(tc, "warning_count", "34");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // as if we started again
        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &[]);

        jassert!(tc, "start_offset", "6900");
        jassert!(tc, "start_line", "69");
        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "65");
        jassert!(tc, "warning_count", "64");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");
    }

    // successive runs simulation with no save threshold
    if testcases.is_empty() || testcases.contains(&"successive_runs_nosave_thresholds") {
        let mut tc = TestCase::new("successive_runs_nosave_thresholds", &mut nb_testcases);

        // first run
        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // simulate a logfile growth
        tc.grow();

        // second run
        let rc = tc.run(&opts, &[]);
        // jassert!(tc, "start_offset", "20100");
        // jassert!(tc, "start_line", "201");
        jassert!(tc, "last_offset", "40200");
        jassert!(tc, "last_line", "402");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");
    }

    // successive runs simulation with save threshold
    if testcases.is_empty() || testcases.contains(&"successive_runs_save_thresholds") {
        let mut tc = TestCase::new("successive_runs_save_thresholds", &mut nb_testcases);

        // first run
        Config::default()
            .set_tag("options", "savethresholds")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // simulate a logfile growth
        tc.grow();

        // second run
        let rc = tc.run(&opts, &[]);
        jassert!(tc, "last_offset", "40200");
        jassert!(tc, "last_line", "402");
        jassert!(tc, "critical_count", "198");
        jassert!(tc, "warning_count", "196");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");
    }

    // successive runs rotation with no save threshold
    if testcases.is_empty() || testcases.contains(&"rotate_nosave_thresholds") {
        let mut tc = TestCase::new("rotate_nosave_thresholds", &mut nb_testcases);

        // first run
        Config::default()
            .set_tag("options", "stopat=70")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "6900");
        jassert!(tc, "last_line", "69");
        jassert!(tc, "critical_count", "34");
        jassert!(tc, "warning_count", "34");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // simulate a logfile rotation
        tc.rotate();

        // second run
        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &[]);
        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");
    }

    // successive runs rotation with save threshold
    if testcases.is_empty() || testcases.contains(&"rotate_save_thresholds") {
        let mut tc = TestCase::new("rotate_save_thresholds", &mut nb_testcases);

        // first run
        Config::default()
            .set_tag("options", "stopat=70")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        jassert!(tc, "last_offset", "6900");
        jassert!(tc, "last_line", "69");
        jassert!(tc, "critical_count", "34");
        jassert!(tc, "warning_count", "34");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // simulate a logfile rotation
        tc.rotate();

        // second run
        Config::default()
            .set_tag("options", "savethresholds,protocol")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &[]);
        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "198");
        jassert!(tc, "warning_count", "196");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");
    }

    // prescript
    #[cfg(target_family = "unix")]
    if testcases.is_empty() || testcases.contains(&"prescript") {
        let mut tc = TestCase::new("prescript", &mut nb_testcases);

        // first run
        Config::from_file("./tests/integration/config/prescript.yml")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        assert_eq!(rc.0, 3);
        jassert!(rc, "UNKNOWN");
    }

    // callback call
    #[cfg(target_family = "unix")]
    if testcases.is_empty() || testcases.contains(&"callback_domain") {
        let mut tc = TestCase::new("callback_domain", &mut nb_testcases);
        Config::default()
            .set_tag("options", "runcallback")
            .set_tag("path", &tc.logfile)
            .replace_tag(
                "address",
                "domain",
                "./tests/integration/tmp/generated.sock
                ",
            )
            .save_as(&tc.config_file);

        // create UDS server
        let addr = "./tests/integration/tmp/generated.sock";
        let _ = std::fs::remove_file(&addr);

        let child = std::thread::spawn(move || {
            // create a listener
            let listener = std::os::unix::net::UnixListener::bind(addr).unwrap();
            match listener.accept() {
                Ok((mut socket, _addr)) => {
                    // set short timeout
                    socket
                        .set_read_timeout(Some(std::time::Duration::new(3, 0)))
                        .expect("Couldn't set read timeout");

                    let mut nb_received = 0;

                    // loop to receive data
                    loop {
                        let json = JSONStream::get_json_from_stream(&mut socket);
                        if json.is_err() {
                            break;
                        }

                        nb_received += 1;

                        let j = json.unwrap();

                        // all asserts here
                        assert_eq!(j.args, &["arg1", "arg2", "arg3"]);

                        // globals are only sent once
                        if nb_received == 1 {
                            assert_eq!(
                                j.global.as_ref().unwrap().get("CLF_firstname").unwrap(),
                                "Al"
                            );
                            assert_eq!(
                                j.global.as_ref().unwrap().get("CLF_lastname").unwrap(),
                                "Pacino"
                            );
                            assert_eq!(
                                j.global.as_ref().unwrap().get("CLF_profession").unwrap(),
                                "actor"
                            );
                            assert_eq!(
                                j.global.as_ref().unwrap().get("CLF_city").unwrap(),
                                "Los Angeles"
                            );
                        }

                        assert_eq!(j.vars.get("CLF_NB_CG").unwrap().as_u64(), 3);
                        assert!(j
                            .vars
                            .get("CLF_LINE")
                            .unwrap()
                            .as_str()
                            .contains("generated for tests"));

                        let line_number: u64 = j.vars.get("CLF_LINE_NUMBER").unwrap().as_u64();
                        assert!(line_number <= 201);

                        let cg1: usize = j.vars.get("CLF_CG_1").unwrap().as_str().parse().unwrap();
                        assert!(cg1 <= 201);

                        let cg2: usize = j.vars.get("CLF_CG_2").unwrap().as_str().parse().unwrap();
                        assert!(cg2 <= 99999);
                        assert!(cg2 >= 10000);

                        //println!("json={:#?}", j);
                    }
                }
                Err(e) => panic!("couldn't get client: {:?}", e),
            }
        });

        // wait a little before calling
        let ten_millis = std::time::Duration::from_millis(10);
        std::thread::sleep(ten_millis);

        let rc = tc.run(&opts, &["-d"]);
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        let _res = child.join();
    }

    // callback call
    if testcases.is_empty() || testcases.contains(&"callback_tcp") {
        let mut tc = TestCase::new("callback_tcp", &mut nb_testcases);
        Config::default()
            .set_tag("options", "runcallback")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);

        // create UDS server
        let addr = "127.0.0.1:8999";

        let child = std::thread::spawn(move || {
            // create a listener
            let listener = std::net::TcpListener::bind(addr).unwrap();
            match listener.accept() {
                Ok((mut socket, _addr)) => {
                    // set short timeout
                    socket
                        .set_read_timeout(Some(std::time::Duration::new(3, 0)))
                        .expect("Couldn't set read timeout");

                    let mut nb_received = 0;

                    // loop to receive data
                    loop {
                        let json = JSONStream::get_json_from_stream(&mut socket);
                        if json.is_err() {
                            break;
                        }

                        nb_received += 1;

                        let j = json.unwrap();

                        // all asserts here
                        assert_eq!(j.args, &["arg1", "arg2", "arg3"]);

                        // globals are only sent once
                        if nb_received == 1 {
                            assert_eq!(
                                j.global.as_ref().unwrap().get("CLF_firstname").unwrap(),
                                "Al"
                            );
                            assert_eq!(
                                j.global.as_ref().unwrap().get("CLF_lastname").unwrap(),
                                "Pacino"
                            );
                            assert_eq!(
                                j.global.as_ref().unwrap().get("CLF_profession").unwrap(),
                                "actor"
                            );
                            assert_eq!(
                                j.global.as_ref().unwrap().get("CLF_city").unwrap(),
                                "Los Angeles"
                            );
                        }

                        assert_eq!(j.vars.get("CLF_NB_CG").unwrap().as_u64(), 3);
                        assert!(j
                            .vars
                            .get("CLF_LINE")
                            .unwrap()
                            .as_str()
                            .contains("generated for tests"));

                        let line_number: u64 = j.vars.get("CLF_LINE_NUMBER").unwrap().as_u64();
                        assert!(line_number <= 201);

                        let cg1: usize = j.vars.get("CLF_CG_1").unwrap().as_str().parse().unwrap();
                        assert!(cg1 <= 201);

                        let cg2: usize = j.vars.get("CLF_CG_2").unwrap().as_str().parse().unwrap();
                        assert!(cg2 <= 99999);
                        assert!(cg2 >= 10000);

                        //println!("json={:#?}", j);
                    }
                }
                Err(e) => panic!("couldn't get client: {:?}", e),
            }
        });

        // wait a little before calling
        let ten_millis = std::time::Duration::from_millis(10);
        std::thread::sleep(ten_millis);

        let rc = tc.run(&opts, &["-d"]);
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        let _res = child.join();
    }

    // error during callback exec
    if testcases.is_empty() || testcases.contains(&"callback_error") {
        let mut tc = TestCase::new("callback_error", &mut nb_testcases);
        Config::default()
            .set_tag("options", "runcallback")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);

        // create UDS server
        let addr = "127.0.0.1:8999";

        let child = std::thread::spawn(move || {
            // create a listener and stop to simulate a listener
            let listener = std::net::TcpListener::bind(addr).unwrap();
            match listener.accept() {
                Ok((mut socket, _addr)) => {
                    // set short timeout
                    socket
                        .set_read_timeout(Some(std::time::Duration::new(3, 0)))
                        .expect("Couldn't set read timeout");

                    // loop to receive data
                    loop {
                        let json = JSONStream::get_json_from_stream(&mut socket);
                        if json.is_err() {
                            break;
                        }
                        let j = json.unwrap();

                        let line_number: u64 = j.vars.get("CLF_LINE_NUMBER").unwrap().as_u64();
                        if line_number == 7 {
                            break;
                        };
                    }
                }
                Err(e) => panic!("couldn't get client: {:?}", e),
            }
        });

        // wait a little before calling
        let timeout = std::time::Duration::from_millis(100);
        std::thread::sleep(timeout);

        let rc = tc.run(&opts, &["-d"]);
        jassert!(tc, "last_offset", "700");
        jassert!(tc, "last_line", "7");
        jassert!(tc, "critical_count", "4");
        jassert!(tc, "warning_count", "3");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "7");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        let _res = child.join();
    }

    // call presecript echo domain and send JSON data
    #[cfg(target_family = "unix")]
    if testcases.is_empty() || testcases.contains(&"echodomain") {
        let mut tc = TestCase::new("echodomain", &mut nb_testcases);
        Config::from_file("./tests/integration/config/echodomain.yml")
            .set_tag("options", "runcallback")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let _ = tc.run(&opts, &["-d"]);

        // check resulting file created from running script
        let data: String = std::fs::read_to_string(&tc.tmpfile)
            .expect(&format!("unable to open file {}", &tc.tmpfile));
        assert!(data.contains(&"tests/integration/tmp/echodomain.log"));
    }

    // call presecript echo domain and send JSON data
    #[cfg(target_family = "unix")]
    if testcases.is_empty() || testcases.contains(&"echotcp") {
        let mut tc = TestCase::new("echotcp", &mut nb_testcases);
        Config::from_file("./tests/integration/config/echotcp.yml")
            .set_tag("options", "runcallback")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let _ = tc.run(&opts, &["-d"]);

        // check resulting file created from running script
        let data: String = std::fs::read_to_string(&tc.tmpfile)
            .expect(&format!("unable to open file {}", &tc.tmpfile));
        assert!(data.contains(&"tests/integration/tmp/echotcp.log"));
    }

    println!("Number of test cases executed: {}", nb_testcases - 1);
}
