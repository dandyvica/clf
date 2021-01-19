use clap::{App, Arg};

mod testcase;
use testcase::{Config, FakeLogfile, JSONStream, Target, TestCase};

// list of cli options
#[derive(Debug)]
struct Options {
    mode: Target,
    verbose: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            mode: Target::debug,
            verbose: false,
        }
    }
}

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
        .get_matches();

    let mut opts = Options::default();

    opts.mode = matches.value_of_t("mode").unwrap_or(Target::debug);
    opts.verbose = matches.is_present("verbose");

    //println!("options={:?}", opts);

    // create tmp directory if not present
    if !std::path::Path::new("./tests/integration/tmp").exists() {
        std::fs::create_dir("./tests/integration/tmp").expect("unable to create tmp dir");
    }

    // generate a dummy logfile
    FakeLogfile::init();

    //------------------------------------------------------------------------------------------------
    // command line flags
    //------------------------------------------------------------------------------------------------

    // call help
    {
        let tc = TestCase::new("help");
        let rc = tc.exec(&opts.mode, &["--help"]);

        assert_eq!(rc, 0);
    }

    // missing argument
    {
        let tc = TestCase::new("missing_argument");
        let rc = tc.exec(&opts.mode, &["--syntax-check"]);

        assert_eq!(rc, 2);
    }

    // good YAML syntax
    {
        let tc = TestCase::new("good_syntax");
        let rc = tc.exec(
            &opts.mode,
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
            &opts.mode,
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
            &opts.mode,
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
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

        jassert!(tc, "compression", "uncompressed");
        jassert!(tc, "extension", "log");
        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "0");
        jassert!(tc, "warning_count", "0");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 0);
    }

    // fastforward gzipped
    {
        // zip logfile
        FakeLogfile::gzip(true);

        let mut tc = TestCase::new("fastforward_gzipped");
        Config::default()
            .set_tag("options", "fastforward")
            .set_tag("path", "./tests/integration/logfiles/generated.log.gz")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

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

        // delete what we created because it's used by rotation tests
        FakeLogfile::gzip_delete();
    }

    // logfile missing
    {
        let mut tc = TestCase::new("logfilemissing");

        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", "./tmp/my_foo_file")
            .set_tag("logfilemissing", "critical")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", "./tmp/my_foo_file")
            .set_tag("logfilemissing", "warning")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);
        assert_eq!(rc.0, 1);
        jassert!(rc, "warning");

        Config::default()
            .set_tag("options", "protocol")
            .set_tag("path", "./tmp/my_foo_file")
            .set_tag("logfilemissing", "unknown")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);
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
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

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
        // create utf8 file
        FakeLogfile::create_utf8();

        let mut tc = TestCase::new("utf8");
        Config::from_file("./tests/integration/config/utf8.yml")
            .set_tag("options", "protocol")
            .set_tag("path", "./tests/integration/logfiles/generated_utf8.log")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

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
            .save_as(&tc.config_file);
        let context = "{\"path\":\"./tests/integration/logfiles/generated.log\"}";
        let rc = tc.run(&opts.mode, &["-d", "-x", context]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
    }

    //------------------------------------------------------------------------------------------------
    // list files Linux
    //------------------------------------------------------------------------------------------------
    #[cfg(target_os = "linux")]
    {
        let mut tc = TestCase::new("list_files");
        Config::from_file("./tests/integration/config/list_linux.yml")
            .set_tag("options", "protocol")
            .set_tag("list", r#"["find", "/var/log", "-type", "f", "-name", "*.log"]"#)
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

        assert_eq!(rc.0, 2);
        jassert!(rc, "/var/log");
    }    

    //------------------------------------------------------------------------------------------------
    // ok pattern
    //------------------------------------------------------------------------------------------------
    {
        let mut tc = TestCase::new("ok_pattern");
        Config::from_file("./tests/integration/config/ok_pattern.yml")
            .set_tag("options", "protocol")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "74");
        jassert!(tc, "warning_count", "73");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
    }

    //------------------------------------------------------------------------------------------------
    // thresholds
    //------------------------------------------------------------------------------------------------
    // criticalthreshold
    {
        let mut tc = TestCase::new("thresholds");
        Config::default()
            .set_tag("options", "criticalthreshold=50,warningthreshold=60")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "49");
        jassert!(tc, "warning_count", "38");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");
    }

    // criticalthreshold
    {
        let mut tc = TestCase::new("huge_thresholds");
        Config::default()
            .set_tag("options", "criticalthreshold=1500,warningthreshold=1500")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

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
        Config::default()
            .set_tag("options", "runcallback")
            .replace_tag(
                "address",
                "script",
                "./tests/integration/scripts/echovars.py",
            )
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "197");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");
    }

    // run a script with a threshold
    {
        let mut tc = TestCase::new("script_threshold");
        Config::default()
            .set_tag(
                "options",
                "runcallback,criticalthreshold=50,warningthreshold=60",
            )
            .replace_tag(
                "address",
                "script",
                "./tests/integration/scripts/echovars.py",
            )
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

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
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

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
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &[]);

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
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // simulate a logfile growth
        FakeLogfile::grow();

        // second run
        let rc = tc.run(&opts.mode, &[]);
        jassert!(tc, "start_offset", "20100");
        jassert!(tc, "start_line", "201");
        jassert!(tc, "last_offset", "40200");
        jassert!(tc, "last_line", "402");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // regenerate standard logfile
        FakeLogfile::init();
    }

    // successive runs simulation with save threshold
    {
        let mut tc = TestCase::new("successive_runs_save_thresholds");

        // first run
        Config::default()
            .set_tag("options", "savethresholds")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // simulate a logfile growth
        FakeLogfile::grow();

        // second run
        let rc = tc.run(&opts.mode, &[]);
        jassert!(tc, "last_offset", "40200");
        jassert!(tc, "last_line", "402");
        jassert!(tc, "critical_count", "198");
        jassert!(tc, "warning_count", "196");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // regenerate standard logfile
        FakeLogfile::init();
    }

    // successive runs rotation with no save threshold
    {
        let mut tc = TestCase::new("rotate_nosave_thresholds");

        // first run
        Config::default()
            .set_tag("options", "stopat=70")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

        jassert!(tc, "last_offset", "6900");
        jassert!(tc, "last_line", "69");
        jassert!(tc, "critical_count", "34");
        jassert!(tc, "warning_count", "34");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // simulate a logfile rotation
        FakeLogfile::rotate();

        // second run
        Config::default()
            .set_tag("options", "protocol")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &[]);
        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "99");
        jassert!(tc, "warning_count", "98");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // cleanup
        FakeLogfile::gzip_delete();
    }

    // successive runs rotation with save threshold
    {
        let mut tc = TestCase::new("rotate_save_thresholds");

        // first run
        Config::default()
            .set_tag("options", "stopat=70")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

        jassert!(tc, "last_offset", "6900");
        jassert!(tc, "last_line", "69");
        jassert!(tc, "critical_count", "34");
        jassert!(tc, "warning_count", "34");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // simulate a logfile rotation
        FakeLogfile::rotate();

        // second run
        Config::default()
            .set_tag("options", "savethresholds,protocol")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &[]);
        jassert!(tc, "last_offset", "20100");
        jassert!(tc, "last_line", "201");
        jassert!(tc, "critical_count", "198");
        jassert!(tc, "warning_count", "196");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        // cleanup
        FakeLogfile::gzip_delete();
    }

    // callback call
    #[cfg(target_family = "unix")]
    {
        let mut tc = TestCase::new("callback_call");
        Config::default()
            .set_tag("options", "runcallback")
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

                    // loop to receive data
                    loop {
                        let json = JSONStream::get_json_from_stream(&mut socket);
                        if json.is_err() {
                            break;
                        }

                        let j = json.unwrap();

                        // all asserts here
                        assert_eq!(j.args, &["arg1", "arg2", "arg3"]);

                        assert_eq!(j.global.get("CLF_firstname").unwrap(), "Al");
                        assert_eq!(j.global.get("CLF_lastname").unwrap(), "Pacino");
                        assert_eq!(j.global.get("CLF_profession").unwrap(), "actor");
                        assert_eq!(j.global.get("CLF_city").unwrap(), "Los Angeles");

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

        let rc = tc.run(&opts.mode, &["-d"]);
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");

        let _res = child.join();
    }
}
