//! Identies uniquely a file in a filesystem. This is how we can detect a file has been archived
//!
#[cfg(target_family = "unix")]
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::misc::error::AppError;

// Combination a an inod and a dev
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Signature {
    inode: u64,
    dev: u64,
}

// impl Signature {
//     // simple getters
//     #[inline(always)]
//     pub fn inode(&self) -> u64 {
//         self.inode
//     }

//     #[inline(always)]
//     pub fn dev(&self) -> u64 {
//         self.dev
//     }
// }

pub trait FileIdentification {
    fn signature(&self) -> Result<Signature, AppError>;
    // fn has_changed(&self, other: &Signature) -> Result<bool, AppError>;
}

impl FileIdentification for PathBuf {
    // get inode and dev from file
    #[cfg(target_family = "unix")]
    fn signature(&self) -> Result<Signature, AppError> {
        let metadata = self.metadata()?;

        Ok(Signature {
            inode: metadata.ino(),
            dev: metadata.dev(),
        })
    }

    #[cfg(target_family = "windows")]
    fn signature(&self) -> Result<Signature, AppError> {
        unimplemented!("Signature trait not yet implemented for Windows");
    }
}
