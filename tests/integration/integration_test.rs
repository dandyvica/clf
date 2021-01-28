use clap::{App, Arg};

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
        .get_matches();

    let mut opts = Options::default();

    opts.mode = matches.value_of_t("mode").unwrap_or(Target::debug);
    opts.verbose = matches.is_present("verbose");
    opts.clf = matches
        .value_of_t("clf")
        .unwrap_or(opts.mode.path().to_string());

    //println!("options={:?}", opts);

    //------------------------------------------------------------------------------------------------
    // prepare run by creating necessary directories if necessary
    //------------------------------------------------------------------------------------------------
    TestScenario::prepare();

    //------------------------------------------------------------------------------------------------
    // command line flags
    //------------------------------------------------------------------------------------------------

    // call help
    {
        let tc = TestCase::new("help");
        let rc = tc.exec(&opts, &["--help"]);

        assert_eq!(rc, 0);
    }

    // missing argument
    {
        let tc = TestCase::new("missing_argument");
        let rc = tc.exec(&opts, &["--syntax-check"]);

        assert_eq!(rc, 2);
    }

    // good YAML syntax
    {
        let tc = TestCase::new("good_syntax");
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
    {
        let tc = TestCase::new("bad_syntax");
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
    {
        let tc = TestCase::new("show_options");
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
    // fastforward
    {
        let mut tc = TestCase::new("fastforward");
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
    {
        let mut tc = TestCase::new("fastforward_gzipped");
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
    {
        let mut tc = TestCase::new("logfilemissing");

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
    {
        let mut tc = TestCase::new("dummy");
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
    {
        let mut tc = TestCase::new("utf8");
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
    {
        let mut tc = TestCase::new("tera");
        Config::from_file("./tests/integration/config/tera.yml")
            .set_tag("options", "protocol")
            .set_tag("path", &tc.logfile)
            .save_as(&tc.config_file);
        let context = "{\"path\":\"./tests/integration/tmp/generated.log\"}";
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
    {
        let mut tc = TestCase::new("retention");

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

    //------------------------------------------------------------------------------------------------
    // list files Linux
    //------------------------------------------------------------------------------------------------
    #[cfg(target_os = "linux")]
    {
        let mut tc = TestCase::new("list_files");
        tc.multiple_logs();

        Config::from_file("./tests/integration/config/list_linux.yml")
            .set_tag("options", "protocol")
            .set_tag(
                "list",
                r#"["find", "./tests/integration/tmp", "-type", "f", "-name", "list_files.log.*"]"#,
            )
            .save_as(&tc.config_file);
        let rc = tc.run(&opts, &["-d"]);

        assert_eq!(rc.0, 2);
        jassert!(rc, "list_files.log");
    }

    //------------------------------------------------------------------------------------------------
    // ok pattern
    //------------------------------------------------------------------------------------------------
    // ok pattern but runifok = false
    {
        let mut tc = TestCase::new("ok_pattern");
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
    {
        let mut tc = TestCase::new("runifok");
        #[cfg(target_family = "unix")]
        Config::from_file("./tests/integration/config/ok_pattern.yml")
            .set_tag("options", "runcallback,runifok")
            .set_tag("path", &tc.logfile)
            .replace_tag(
                "address",
                "script",
                "./tests/integration/scripts/echovars.py",
            )
            .set_tag("args", "['/tmp/runifok.txt', 'arg2']")
            .save_as(&tc.config_file);
        #[cfg(target_family = "windows")]
        Config::from_file("./tests/integration/config/ok_pattern.yml")
            .set_tag("options", "runcallback")
            .replace_tag("address", "script", "python.exe")
            .set_tag(
                "args",
                r"['.\tests\integration\scripts\echovars.py', 'c:\windows\temp\runifok.txt']",
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
    {
        let mut tc = TestCase::new("thresholds");
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
    {
        let mut tc = TestCase::new("huge_thresholds");
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
    {
        let mut tc = TestCase::new("start_script");
        #[cfg(target_family = "unix")]
        Config::default()
            .set_tag("options", "runcallback")
            .set_tag("path", &tc.logfile)
            .replace_tag(
                "address",
                "script",
                "./tests/integration/scripts/echovars.py",
            )
            .set_tag("args", "['/tmp/start_script.txt', 'arg2']")
            .save_as(&tc.config_file);
        #[cfg(target_family = "windows")]
        Config::default()
            .set_tag("options", "runcallback")
            .replace_tag("address", "script", "python")
            .set_tag(
                "args",
                r"['.\tests\integration\scripts\echovars.py', 'c:\windows\temp\start_script.txt']",
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
    {
        let mut tc = TestCase::new("script_threshold");
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
                "./tests/integration/scripts/echovars.py",
            )
            .set_tag("args", "['/tmp/script_threshold.txt', 'arg2']")
            .save_as(&tc.config_file);
        #[cfg(target_family = "windows")]
        Config::default()
            .set_tag(
                "options",
                "runcallback,criticalthreshold=50,warningthreshold=60",
            )
            .replace_tag("address", "script", "python")
            .set_tag("args", r"['.\tests\integration\scripts\echovars.py', c:\windows\temp\script_threshold.txt']")
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
    {
        let mut tc = TestCase::new("stopat");
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
    {
        let mut tc = TestCase::new("successive_runs_nosave_thresholds");

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
    {
        let mut tc = TestCase::new("successive_runs_save_thresholds");

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
    {
        let mut tc = TestCase::new("rotate_nosave_thresholds");

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
    {
        let mut tc = TestCase::new("rotate_save_thresholds");

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
    {
        let mut tc = TestCase::new("prescript");

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
    {
        let mut tc = TestCase::new("callback_domain");
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

                        assert_eq!(j.vars.get("CLF_NB_CG").unwrap(), "3");
                        assert!(j
                            .vars
                            .get("CLF_LINE")
                            .unwrap()
                            .as_str()
                            .contains("generated for tests"));

                        let line_number: usize =
                            j.vars.get("CLF_LINE_NUMBER").unwrap().parse().unwrap();
                        assert!(line_number <= 201);

                        let cg1: usize = j.vars.get("CLF_CG_1").unwrap().parse().unwrap();
                        assert!(cg1 <= 201);

                        let cg2: usize = j.vars.get("CLF_CG_2").unwrap().parse().unwrap();
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
    {
        let mut tc = TestCase::new("callback_tcp");
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

                        assert_eq!(j.vars.get("CLF_NB_CG").unwrap(), "3");
                        assert!(j
                            .vars
                            .get("CLF_LINE")
                            .unwrap()
                            .as_str()
                            .contains("generated for tests"));

                        let line_number: usize =
                            j.vars.get("CLF_LINE_NUMBER").unwrap().parse().unwrap();
                        assert!(line_number <= 201);

                        let cg1: usize = j.vars.get("CLF_CG_1").unwrap().parse().unwrap();
                        assert!(cg1 <= 201);

                        let cg2: usize = j.vars.get("CLF_CG_2").unwrap().parse().unwrap();
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
}
