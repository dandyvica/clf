use std::path::Path;

use serde::Deserialize;

use crate::error::AppError;

// #macro_rules! is_set {
//     ($settings:var, $field:expr) => {
//         $settings.is_some() && $
//     };
// }

/// Command line settings, which interfere with core search engine. It's different from the
/// `Config` data structure.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// controls the `BufReader` size, with is by default set to 8k
    pub bufreader_size: usize,
}

impl Settings {
    pub fn from_file<P: AsRef<Path>>(settings_file: P) -> Result<Settings, AppError> {
        let file = std::fs::File::open(settings_file)?;

        // load YAML data
        let yaml = serde_yaml::from_reader(file)?;
        Ok(yaml)
    }
}
