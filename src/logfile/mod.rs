//! All structure for reading, looking for a regex match and calling a callback.
#[macro_use]
#[warn(clippy::module_inception)]
pub mod logfile;
pub mod compression;
pub mod logfileerror;
pub mod logfileid;
pub mod lookup;
pub mod rundata;
pub mod seeker;
pub mod snapshot;
