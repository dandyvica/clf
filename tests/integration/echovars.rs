use std::env;
use std::fs::OpenOptions;
use std::io::prelude::*;

fn main() {
    let vars: Vec<(String, String)> = std::env::vars()
        .filter(|x| x.0.as_str().starts_with("CLF_"))
        .collect();

    let args: Vec<String> = env::args().collect();

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&args[1])
        .unwrap();

    let _ = write!(file, "{}-{:?}\n", std::process::id(), vars);
}
