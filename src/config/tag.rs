//! Contains the configuration of each tag.
use serde::Deserialize;

use crate::config::{
    callback::{Callback, CallbackHandle, ChildData},
    options::SearchOptions,
    pattern::{PatternMatchResult, PatternSet},
    vars::{GlobalVars, RuntimeVars},
};

use crate::misc::error::AppResult;

use crate::fromstr;

/// This is the core structure which handles data used to search into the logfile. These are
/// gathered and refered to as a tag name.
#[derive(Debug, Deserialize, Clone)]
pub struct Tag {
    /// A name to identify the tag.
    pub name: String,

    /// Tells whether we process this tag or not. Useful for testing purposes.
    #[serde(default = "Tag::default_process")]
    pub process: bool,

    /// A list of options specific to this search. As such options are optional, add a default `serde`
    /// directive.
    #[serde(default = "SearchOptions::default")]
    pub options: SearchOptions,

    /// Script details like path, name, parameters, delay etc to be possibly run for a match.
    pub callback: Option<Callback>,

    /// Patterns to be checked against. These include critical and warning (along with exceptions), ok list of regexes.
    pub patterns: PatternSet,
}

impl Tag {
    /// Returns the regex involved in a match, if any, along with associated the pattern type.
    pub fn is_match(&self, text: &str) -> Option<PatternMatchResult> {
        self.patterns.is_match(text)
    }

    /// Default value for processing a tag
    pub fn default_process() -> bool {
        true
    }

    /// Calls the external callback, by providing arguments, environment variables and path which will be searched for the command.
    pub fn callback_call(
        &self,
        path: Option<&str>,
        global_vars: &GlobalVars,
        runtime_vars: &RuntimeVars,
        handle: &mut CallbackHandle,
    ) -> AppResult<Option<ChildData>> {
        if self.callback.is_some() {
            self.callback
                .as_ref()
                .unwrap()
                .call(path, global_vars, runtime_vars, handle)
        } else {
            Ok(None)
        }
    }
}

// Auto-implement FromStr
fromstr!(Tag);

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_family = "unix")]
    fn tag() {
        let yaml = r#"
name: error
options: "runcallback"
process: false
callback: { 
    script: "tests/scripts/echovars.py",
    args: ['arg1', 'arg2', 'arg3']
}
patterns:
    warning: {
        regexes: [
            'error',
        ],
        exceptions: [
            'STARTTLS'
        ]
    }
        "#;

        let tag: Tag = Tag::from_str(yaml).expect("unable to read YAML");
        assert_eq!(tag.name, "error");
        assert!(tag.options.runcallback);
        assert!(!tag.options.keepoutput);
        assert!(!tag.process);
        let script = std::path::PathBuf::from("tests/scripts/echovars.py");
        assert!(
            matches!(&tag.callback.as_ref().unwrap().callback, crate::config::callback::CallbackType::Script(Some(x)) if x == &script)
        );
        assert_eq!(
            tag.callback.unwrap().args.unwrap(),
            &["arg1", "arg2", "arg3"]
        );
    }
}
