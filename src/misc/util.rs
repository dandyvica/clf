use std::time::{Duration, SystemTime};

use crate::context;
use crate::misc::error::{AppError, AppResult};

/// All constants reside here.

/// A default value for the retention of data in the snapshot file.
pub const DEFAULT_RETENTION: u64 = 86000 * 7;

/// Variable name prefix to be inserted for each variable.
//pub const VAR_PREFIX: &'static str = "CLF_";

/// Default capacity for all `Vec` or `HashMap` pre-allocations
pub const DEFAULT_CONTAINER_CAPACITY: usize = 30;

/// Default capacity for all strings pre-allocations
pub const DEFAULT_STRING_CAPACITY: usize = 1024;

/// We define here the maximum size for the logger file (in Mb).
pub const MAX_LOGGER_SIZE: u64 = 50;

/// Returns the number of seconds for a standard timeout when not defined in the YAML file.
/// Needed by `serde`.
pub const fn default_timeout() -> u64 {
    180
}

/// Default hash buffer size
pub const DEFAULT_HASH_BUFFER_SIZE: usize = 4096;

// default time for waiting to spawned scripts
pub const DEFAULT_SCRIPT_TIMEOUT: u64 = 10;

fn from_epoch() -> AppResult<Duration> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| context!(e, "duration_since() error",))
}

pub fn from_epoch_secs() -> AppResult<u64> {
    let from_epoch = from_epoch()?;
    Ok(from_epoch.as_secs())
}
