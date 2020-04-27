use std::path::PathBuf;

pub trait Usable {
    fn is_usable(&self) -> bool;
}

impl Usable for PathBuf {
    fn is_usable(&self) -> bool {
        self.has_root() && self.exists() && self.is_file()
    }
}
