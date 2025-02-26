use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct Filesystem {
    pub shares: Vec<Share>
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct Share {
    pub source: PathBuf,
    pub tag: String,
    pub write: bool,
}
