use std::thread;

use serde::Deserialize;

use rclf::command::Cmd;

#[test]
fn list_files_shell() {
    let files = Cmd::get_list("find", &["/var/log", "-ctime", "+1"]).expect("error listing files");
    assert!(files.len() > 10);
}
#[test]
fn list_files_command() {
    let files = Cmd::get_list("tests/assets/ls.sh", &[]).expect("error listing files");
    assert!(files.len() > 10);
    println!("{:?}", files);
}
#[test]
fn spawn() {
    let file = std::fs::File::open("tests/assets/cmd.yml").expect("unable to open config.yml");
    let cmd: Cmd = serde_yaml::from_reader(file).expect("unable to load YAML");

    let handle = cmd.spawn(None).expect("unable to call script");
    let _ = handle.join();
}
