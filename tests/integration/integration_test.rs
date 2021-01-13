use std::{env::temp_dir, fmt::write, process::Command};

use clap::{App, Arg};

mod testcase;
use testcase::{Config, Target, TestCase};

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

    println!("options={:?}", opts);

    // create tmp directory if not present
    if !std::path::Path::new("./tests/integration/tmp").exists() {
        std::fs::create_dir("./tests/integration/tmp").expect("unable to create tmp dir");
    }

    //------------------------------------------------------------------------------------------------
    // command line flags
    //------------------------------------------------------------------------------------------------

    // TC1: call help
    {
        let tc = TestCase::new("help");
        let rc = tc.exec(&opts.mode, &["--help"]);

        assert_eq!(rc, 0);
    }

    // TC2: missing argument
    {
        let tc = TestCase::new("missing_argument");
        let rc = tc.exec(&opts.mode, &["--syntax-check"]);

        assert_eq!(rc, 2);
    }

    // TC3: good YAML syntax
    {
        let tc = TestCase::new("good_syntax");
        let rc = tc.exec(
            &opts.mode,
            &[
                "--config",
                "./tests/integration/config/tc3.yml",
                "--syntax-check",
            ],
        );

        assert_eq!(rc, 0);
    }

    // TC4: bad YAML syntax
    {
        let tc = TestCase::new("bad_syntax");
        let rc = tc.exec(
            &opts.mode,
            &[
                "--config",
                "./tests/intergration/config/tc4.yml",
                "--syntax-check",
            ],
        );

        assert_eq!(rc, 2);
    }

    // TC5: show options
    {
        let tc = TestCase::new("show_options");
        let rc = tc.exec(
            &opts.mode,
            &[
                "--config",
                "./tests/integration/config/tc3.yml",
                "--syntax-check",
                "--show-options",
            ],
        );

        assert_eq!(rc, 0);
    }

    //------------------------------------------------------------------------------------------------
    // genuine search
    //------------------------------------------------------------------------------------------------

    // TC6: fastforward
    {
        let mut tc = TestCase::new("fastforward");
        Config::default()
            .set_tag("options", "fastforward")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &[]);

        jassert!(tc, "compression", "uncompressed");
        jassert!(tc, "extension", "log");
        jassert!(tc, "last_offset", "197326");
        jassert!(tc, "last_line", "999");
        jassert!(tc, "critical_count", "0");
        jassert!(tc, "warning_count", "0");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 0);
    }

    // fastforward gzipped
    {
        let mut tc = TestCase::new("fastforward_gzipped");
        Config::default()
            .set_tag("options", "fastforward")
            .set_tag("path", "./tests/integration/logfiles/access_simple.log.gz")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &[]);

        jassert!(tc, "compression", "gzip");
        jassert!(tc, "extension", "gz");
        jassert!(tc, "last_offset", "197326");
        jassert!(tc, "last_line", "999");
        jassert!(tc, "critical_count", "0");
        jassert!(tc, "warning_count", "0");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 0);
        jassert!(rc, "OK");
    }

    //------------------------------------------------------------------------------------------------
    // thresholds
    //------------------------------------------------------------------------------------------------
    // criticalthreshold
    {
        let mut tc = TestCase::new("thresholds");
        Config::default()
            .set_tag("options", "criticalthreshold=400,warningthreshold=10")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

        jassert!(tc, "last_offset", "197326");
        jassert!(tc, "last_line", "999");
        jassert!(tc, "critical_count", "40");
        jassert!(tc, "warning_count", "18");
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

        jassert!(tc, "last_offset", "197326");
        jassert!(tc, "last_line", "999");
        jassert!(tc, "critical_count", "0");
        jassert!(tc, "warning_count", "0");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 0);
        jassert!(rc, "OK");
    }

    // savethresholds
    // {
    //     let mut tc = TestCase::new("savethresholds");
    //     Config::default()
    //         .set_tag("options", "savethresholdscriticalthreshold=1500,warningthreshold=1500")
    //         .save_as(&tc.config_file);
    //     let rc = tc.run(&opts.mode, &["-d"]);

    //     jassert!(tc, "last_offset", "197326");
    //     jassert!(tc, "last_line", "999");
    //     jassert!(tc, "critical_count", "0");
    //     jassert!(tc, "warning_count", "0");
    //     jassert!(tc, "ok_count", "0");
    //     jassert!(tc, "exec_count", "0");
    //     assert_eq!(rc.0, 0);
    //     jassert!(rc, "OK");
    // }    

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

        jassert!(tc, "last_offset", "197326");
        jassert!(tc, "last_line", "999");
        jassert!(tc, "critical_count", "440");
        jassert!(tc, "warning_count", "28");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "468");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");
    }

    // run a script with a threshold
    {
        let mut tc = TestCase::new("script_threshold");
        Config::default()
            .set_tag("options", "runcallback,criticalthreshold=400,warningthreshold=1500")
            .replace_tag("address", "script", "./tests/integration/scripts/echovars.py")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

        jassert!(tc, "last_offset", "197326");
        jassert!(tc, "last_line", "999");
        jassert!(tc, "critical_count", "40");
        jassert!(tc, "warning_count", "0");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "41");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");
    }

    // stop at
    {
        let mut tc = TestCase::new("stopat");
        Config::default()
            .set_tag("options", "stopat=900")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

        jassert!(tc, "last_offset", "177955");
        jassert!(tc, "last_line", "900");
        jassert!(tc, "critical_count", "395");
        jassert!(tc, "warning_count", "23");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");
    }

    // successive runs simulation
    {
        let mut tc = TestCase::new("successive_runs");

        // first run
        Config::default()
            .set_tag("options", "stopat=900")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &["-d"]);

        // second run
        Config::default()
            .set_tag("options", "protocol")
            .save_as(&tc.config_file);
        let rc = tc.run(&opts.mode, &[]);


        jassert!(tc, "last_offset", "197326");
        jassert!(tc, "last_line", "999");
        jassert!(tc, "critical_count", "44");
        jassert!(tc, "warning_count", "5");
        jassert!(tc, "ok_count", "0");
        jassert!(tc, "exec_count", "0");
        assert_eq!(rc.0, 2);
        jassert!(rc, "CRITICAL");
    }

}
