use std::{env::temp_dir, fmt::write, process::Command};

use serde_json::{Result, Value};

// used to specify debug or release configuration
enum Target {
    Debug,
    Release,
}

impl Target {
    fn path(&self) -> &str {
        match self {
            Target::Debug => "./target/debug/clf",
            Target::Release => "./target/release/clf",
        }
    }
}

// keep configuration data
struct Config {
    yaml: String,
}

// a struct for holding test case details
struct TestCase {
    name: String,
    tag: String,
    snap: String,
    config: String,
}

impl TestCase {
    fn new(tag: &str, name: &str) -> Self {
        println!("running test case: {}: {}", tag, name);
        TestCase { 
            name: name.to_string(),
            tag: tag.to_string(),
            snap: format!("./tests/integration/tmp/{}.json", tag),
            config: format!("./tests/integration/config/{}.yml", tag)
         }
    }

    // call CLF executable with optional arguments
    fn run(&self, target: &Target, optargs: &[&str]) -> (i32, String) {
        let clf = target.path();

        let output = Command::new(clf)
            .args(&["-c", &self.config, "-p", &self.snap, "-g", "Trace"])
            .args(optargs)
            .output()
            .expect("unable to start clf");

        let s = String::from_utf8_lossy(&output.stdout);
        
        (output.status.code().unwrap(), s.to_string())
    }

    // call CLF executable with arguments
    fn exec(&self, target: &Target, args: &[&str]) -> i32 {
        let clf = target.path();
        let mut output = Command::new(clf)
            .args(args)
            .output()
            .expect("unable to start clf");
        output.status.code().unwrap()
    }

    // extract JSON data
    fn get_json(&self) {

    }
}

fn main() {
    let mode = Target::Debug;

    //------------------------------------------------------------------------------------------------
    // command line flags 
    //------------------------------------------------------------------------------------------------

    // TC1: call help
    {
        let tc = TestCase::new("tc1", "with help");
        let rc = tc.exec(&mode, &["--help"]);

        assert_eq!(rc, 0);
    }

    // TC2: missing argument
    {
        let tc = TestCase::new("tc2", "missing argument");
        let rc = tc.exec(&mode, &["--syntax-check"]);

        assert_eq!(rc, 2);
    }

    // TC3: good YAML syntax
    {
        let tc = TestCase::new("tc3", "good YAML syntax");
        let rc = tc.exec(&mode, &["--config", "./tests/integration/config/tc3.yml", "--syntax-check"]);

        assert_eq!(rc, 0);
    }

    // TC4: bad YAML syntax
    {
        let tc = TestCase::new("tc4", "bad YAML syntax");
        let rc = tc.exec(&mode, &["--config", "./tests/intergration/config/tc4.yml", "--syntax-check"]);

        assert_eq!(rc, 2);
    }

    // TC5: show options
    {
        let tc = TestCase::new("tc5", "show options");
        let rc = tc.exec(
            &mode,
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
        let tc = TestCase::new("tc6", "simple run");
        let rc = tc.run(&mode, &[]);

        let file = std::fs::File::open(&tc.snap).expect(&format!("unable to open snapshot {}", &tc.snap));
        let v: Value = serde_json::from_reader(file).unwrap();

        let keys: Vec<_> = v["snapshot"].as_object().unwrap().keys().collect();
        let logfile = keys[0];

        let v1 = v["snapshot"].as_object().unwrap().get(logfile).unwrap();
        let id = v1.as_object().unwrap().get("id").unwrap().as_object().unwrap();
        println!("{:#?}", id);

        let run_data = v1.as_object().unwrap().get("run_data").unwrap().as_object().unwrap().get("tag1").unwrap().as_object().unwrap();
        println!("{:#?}", run_data);
        assert_eq!(rc.0, 0);
    }
}
