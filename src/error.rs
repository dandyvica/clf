//! List of errors from the `clf`executable. Different from the `rclf` crate error module.

/// All non-Nagios error codes
#[allow(non_camel_case_types)]
pub enum AppExitCode {
    /// Error during loading of the YAML file.
    CONFIG_ERROR = 101,

    /// When just reading the config file
    CONFIG_CHECK = 102,

    /// Error when creating logger file
    LOGGER_ERROR = 103,

    /// Error when deleting snapshot file
    SNAPSHOT_DELETE_ERROR = 104,

    /// Error when saving snapshot file
    SNAPSHOT_SAVE_ERROR = 105,

    /// Error when reading stdin
    STDIN_ERROR = 106,

    /// Exit when showing options
    SHOW_OPTIONS = 107,

    /// Error converting to an integer
    ERROR_CONV = 108,

    /// IO error on config file
    CONFIG_IO_ERROR = 109,
}
