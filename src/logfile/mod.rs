//! The main crate containing all necessary structures and traits for reading and searching
//! a logfile for patterns.
#[macro_use]
pub mod logfile;
pub mod compression;
pub mod signature;
pub mod snapshot;
