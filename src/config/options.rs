//! Configuration options which apply only to a search.
use std::convert::TryFrom;

use serde::Deserialize;

use crate::misc::error::{AppCustomErrorKind, AppError};

/// A list of options which are specific to a search. They might or might not be used. If an option is not present, it's deemed false.
/// By default, all options are either false, or use the default corresponding type.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(try_from = "String")]
pub struct SearchOptions {
    /// If `true`, the defined script will be run a first match.
    pub runcallback: bool,

    /// If `true`, the matching line will be saved in an output file.
    // TODO:
    pub keepoutput: bool,

    /// If `true`, the logfile will be search from the beginning, regardless of any saved offset.
    pub rewind: bool,

    /// a number which denotes how many lines have to match a pattern until they are considered a critical error
    pub criticalthreshold: u64,

    /// a number which denotes how many lines have to match a pattern until they are considered a warning error
    pub warningthreshold: u64,

    // controls whether the matching lines are written to a protocol file for later investigation
    // TODO:
    pub protocol: bool,

    /// controls whether the hit counter will be saved between the runs.
    /// If yes, hit numbers are added until a threshold is reached (criticalthreshold).
    /// Otherwise the run begins resetting counters
    pub savethresholdcount: bool,

    /// controls whether an error is propagated through successive runs of check_logfiles.
    /// Once an error was found, the exitcode will be non-zero until an okpattern resets it or until
    /// the error expires after <second> seconds. Do not use this option until you know exactly what you do
    pub sticky: u16,

    /// Moves to the end of the file for the first read, if the file has not been yet read.
    pub fastforward: bool,

    /// The number of times a potential script will be called, at most.
    pub runlimit: u64,

    /// truncate the read line at specified value before lookup
    pub truncate: usize,

    /// Stop processing of the logfile at this specific line number
    pub stopat: u64,
}

/// Convenient macro to add a boolean option
macro_rules! add_bool_option {
    ($v:ident, $opt:ident, $($bool_option:ident),*) => (
        $(
          if $v.contains(&stringify!($bool_option)) {
            $opt.$bool_option = true;
        }
        )*
    );
}

/// Convenient macro to add an integer or string option.
macro_rules! add_typed_option {
    // add non-boolean option if any. It converts to the target type
    ($x:ident, $tag:ident, $opt:ident, $type:ty) => {
        // `stringify!` will convert the expression *as it is* into a string.
        if $x[0] == stringify!($tag) {
            $opt.$tag = $x[1].parse::<$type>().unwrap();
        }
    };
}

/// Converts a list of comma-separated options to a `SearchOptions` structure.
impl TryFrom<String> for SearchOptions {
    type Error = AppError;

    fn try_from(option_list: String) -> Result<Self, Self::Error> {
        // list of valid options
        const VALID_OPTIONS: &[&str] = &[
            "runcallback",
            "keepoutput",
            "rewind",
            "criticalthreshold",
            "warningthreshold",
            "protocol",
            "savethresholdcount",
            "sticky",
            "fastforward",
            "runlimit",
            "truncate",
            "stopat",
        ];

        // create a default options structure
        let mut opt = SearchOptions::default();

        // runlimit is special
        opt.runlimit = std::u64::MAX;

        // convert the input list to a vector
        let opt_list: Vec<_> = option_list.split(',').map(|x| x.trim()).collect();

        // checks if there're any invalid arguments
        for opt in &opt_list {
            if VALID_OPTIONS.iter().all(|x| !opt.contains(x)) {
                return Err(AppError::new_custom(
                    AppCustomErrorKind::UnsupportedSearchOption,
                    &format!("search option: {}  is not supported", opt),
                ));
            }
        }

        // use Rust macro to add bool options if any
        add_bool_option!(
            opt_list,
            opt,
            runcallback,
            rewind,
            keepoutput,
            savethresholdcount,
            protocol,
            fastforward
        );

        // other options like key=value if any
        // first build a vector of such options. We first search for = and then split according to '='
        let kv_options: Vec<_> = opt_list.iter().filter(|&x| x.contains('=')).collect();

        // need to test whether we found 'key=value' options
        if !kv_options.is_empty() {
            // this hash will hold key values options
            //let kvh_options: HashMap<String, String> = HashMap::new();

            // now we can safely split
            for kv in &kv_options {
                let splitted_options: Vec<_> = kv.split('=').map(|x| x.trim()).collect();
                let _key = splitted_options[0];
                let _value = splitted_options[1];

                // add additional non-boolean options if any
                add_typed_option!(splitted_options, criticalthreshold, opt, u64);
                add_typed_option!(splitted_options, warningthreshold, opt, u64);
                add_typed_option!(splitted_options, sticky, opt, u16);
                add_typed_option!(splitted_options, runlimit, opt, u64);
                add_typed_option!(splitted_options, truncate, opt, usize);
                add_typed_option!(splitted_options, stopat, opt, u64);

                // special case for this
                // if key == "logfilemissing" {
                //     opt.logfilemissing = LogfileMissing::from_str(value).unwrap();
                // }
            }
        }

        Ok(opt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_options() {
        let opts = SearchOptions::try_from("runcallback, keepoutput, rewind, criticalthreshold=10, warningthreshold=15, protocol, savethresholdcount, sticky=5, runlimit=10, truncate=80".to_string()).unwrap();

        assert!(opts.runcallback);
        assert!(opts.keepoutput);
        assert!(opts.rewind);
        assert!(opts.savethresholdcount);
        assert!(opts.protocol);

        assert_eq!(opts.criticalthreshold, 10);
        assert_eq!(opts.warningthreshold, 15);
        assert_eq!(opts.sticky, 5);
        assert_eq!(opts.criticalthreshold, 10);
        assert_eq!(opts.runlimit, 10);
        assert_eq!(opts.truncate, 80);
        //assert_eq!(&opts.logfilemissing.unwrap(), "foo");
    }
}
