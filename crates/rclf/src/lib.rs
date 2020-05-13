//! The main crate containing all necessary structures and traits for reading and searching
//! a logfile for patterns.
#[macro_use]
pub mod error;
pub mod config;
pub mod logfile;
pub mod variables;
pub mod pattern;
pub mod snapshot;
pub mod command;
pub mod util;
