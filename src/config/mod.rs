//! The main crate containing all necessary structures and traits for reading and searching
//! a logfile for patterns.
#[macro_use]
pub mod callback;
pub mod config;
//pub mod archive;
pub mod global;
pub mod logfiledef;
pub mod logsource;
pub mod options;
pub mod pattern;
pub mod script;
pub mod search;
pub mod tag;
pub mod vars;
