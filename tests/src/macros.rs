// a macro for creating a new text file from a string
#[macro_export]
macro_rules! create_file {
    ($file_name:literal, $data:literal) => {{
        let data = $data;
        let _ = std::fs::create_dir("tests/tmp/");
        let data_file = concat!("tests/tmp/", $file_name);
        std::fs::write(&data_file, data).expect("Unable to write file");
        data_file
    }};
}
