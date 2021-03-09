fn main() {
    #[cfg(target_family = "windows")]
    {
        let path = std::env::var("PATH").expect("unable to fetch %PATH%");
        let new_path = format!(r"{};.\src\windows", path);
        std::env::set_var("PATH", new_path);
    }
}
