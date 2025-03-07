use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(default)]
pub struct Filesystem {
    pub shares: Vec<Share>,
    pub disks: Vec<Disk>,
}

#[derive(Default, Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(default)]
pub struct Share {
    pub source: PathBuf,
    pub tag: String,
    pub write: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(default)]
pub struct Disk {
    pub source: PathBuf,
    pub tag: String,
    pub write: bool,
    pub create: bool,
    pub size: u64,
    pub format: Format,
}

impl Default for Disk {
    fn default() -> Self {
        Self {
            source: PathBuf::default(),
            tag: String::default(),
            write: true,
            create: true,
            size: u64::default(),
            format: Format::Qcow2,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum Format {
    #[serde(rename = "qcow2")]
    Qcow2,
    #[serde(rename = "raw")]
    Raw,
}

impl ToString for Format {
    fn to_string(&self) -> String {
        match self {
            Self::Qcow2 => "qcow2",
            Self::Raw => "raw",
        }.to_string()
    }
}
