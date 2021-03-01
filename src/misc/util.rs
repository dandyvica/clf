use std::time::{Duration, SystemTime};

use crate::context;
use crate::misc::error::{AppError, AppResult};

/// All constants reside here.

/// A default value for the retention of data in the snapshot file.
pub const DEFAULT_RETENTION: u64 = 86000 * 7;

/// Default capacity for all `Vec` or `HashMap` pre-allocations
pub const DEFAULT_CONTAINER_CAPACITY: usize = 30;

/// Default capacity for all strings pre-allocations
pub const DEFAULT_STRING_CAPACITY: usize = 1024;

/// We define here the maximum size for the logger file (in Mb).
pub const MAX_LOGGER_SIZE: u64 = 50;

/// Default hash buffer size
pub const DEFAULT_HASH_BUFFER_SIZE: usize = 4096;

// default time for waiting to spawned scripts
pub const DEFAULT_SCRIPT_TIMEOUT: u64 = 10;

// default write socket timeout
pub const DEFAULT_WRITE_TIMEOUT: u64 = 5;

// to save some string allocation, we can define a list of capture groups variables upfront
pub const CAPTURE_GROUPS: &'static [&'static str] = &[
    "CLF_CG_0",
    "CLF_CG_1",
    "CLF_CG_2",
    "CLF_CG_3",
    "CLF_CG_4",
    "CLF_CG_5",
    "CLF_CG_6",
    "CLF_CG_7",
    "CLF_CG_8",
    "CLF_CG_9",
    "CLF_CG_10",
    "CLF_CG_11",
    "CLF_CG_12",
    "CLF_CG_13",
    "CLF_CG_14",
    "CLF_CG_15",
    "CLF_CG_16",
    "CLF_CG_17",
    "CLF_CG_18",
    "CLF_CG_19",
    "CLF_CG_20",
    "CLF_CG_21",
    "CLF_CG_22",
    "CLF_CG_23",
    "CLF_CG_24",
    "CLF_CG_25",
    "CLF_CG_26",
    "CLF_CG_27",
    "CLF_CG_28",
    "CLF_CG_29",
    "CLF_CG_30",
];

pub const CAPTURE_GROUPS_LENGTH: usize = CAPTURE_GROUPS.len();

// utility functions to get the number of seconds from 1/1/1970
fn from_epoch() -> AppResult<Duration> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| context!(e, "duration_since() error",))
}

pub fn from_epoch_secs() -> AppResult<u64> {
    let from_epoch = from_epoch()?;
    Ok(from_epoch.as_secs())
}
