//! List of errors from the `clf`executable. Different from the `rclf` crate error module.

/// List of exit codes

/// Error during loading of the YAML file.
pub const EXIT_CONFIG_ERROR: i32 = 101;

/// When just reading the config file
pub const EXIT_CONFIG_CHECK: i32 = 102;

/// Error when creating logger file
pub const EXIT_LOGGER_ERROR: i32 = 103;

/// Error when deleting snapshot file
pub const EXIT_SNAPSHOT_DELETE_ERROR: i32 = 104;

/// Error when saving snapshot file
pub const EXIT_SNAPSHOT_SAVE_ERROR: i32 = 105;

/// Error when reading stdin
pub const EXIT_STDIN_ERROR: i32 = 106;
