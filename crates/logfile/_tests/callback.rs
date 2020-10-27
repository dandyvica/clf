use rclf::callback::Callback;

#[test]
#[cfg(target_family = "unix")]
fn list_files_command() {
    let files = Callback::get_list("tests/scripts/ls.sh", None).expect("error listing files");
    assert!(files.len() > 10);
    println!("{:?}", files);
}

// #[test]
// fn spawn() {
//     let file = std::fs::File::open("tests/assets/cmd.yml").expect("unable to open config.yml");
//     let cmd: Callback = serde_yaml::from_reader(file).expect("unable to load YAML");

//     let handle = cmd
//         .spawn(None, &RuntimeVariables::new())
//         .expect("unable to call script");
//     let _ = handle.join();
// }
