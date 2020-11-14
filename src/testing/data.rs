// A module for setting up data for unit tests
use regex::Regex;
use serde::Deserialize;

use crate::config::{
    callback::{CallbackHandle, ChildData},
    config::{GlobalOptions, Tag},
    pattern::PatternType,
    variables::Variables,
};

//pub use config::variables::Variables as Variables;

#[derive(Debug, Deserialize)]
pub struct JSONStream {
    pub args: Vec<String>,
    pub vars: Variables,
}

// helper fn to create a dummy Variables struct
pub fn sample_vars() -> Variables {
    // create dummy variables
    let re = Regex::new(r"^([a-z\s]+) (\w+) (\w+) (?P<LASTNAME>\w+)").unwrap();
    let text = "my name is john fitzgerald kennedy, president of the USA";

    let mut vars = Variables::default();
    vars.insert_captures(&re, text);

    vars
}

// help for generating a simple config
pub fn tc_config(opts: &str) -> String {
    let conf = r#"
searches:
    - logfile: tests/logfiles/huge/large_access.log
    tags: 
        - name: tc
        options: "$opts"
        callback: { 
            address: "127.0.0.1:8999",
            args: ['arg1', 'arg2', 'arg3']
        }
        patterns:
            critical: {
            regexes: [
                'GET\s+([/\w]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)',
            ],
            exceptions: [
                'AppleWebKit/537\.36',
            ]
            }
            warning: {
            regexes: [
                'POST\s+([/\w\.]+)\s+HTTP/1\.1"\s+(?P<code>\d+)\s+(?P<length>\d+)'
            ],
            exceptions: [
                '^\d{2,3}    
    "#;

    conf.to_string().replace("$opts", opts)
}