use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Filesystem {
    pub shares: Vec<Share>,
    pub disks: Vec<Disk>,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Share {
    pub source: PathBuf,
    pub tag: String,
    pub write: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Disk {
    pub source: PathBuf,
    pub tag: String,
    pub write: bool,
    pub size: u64,
}

impl Default for Disk {
    fn default() -> Self {
        Self { 
            source: PathBuf::default(), 
            tag: String::default(), 
            write: true, 
            size: u64::default(),
        }
    }
}
