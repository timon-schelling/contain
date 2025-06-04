use std::{fmt::Display, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(default)]
pub struct Filesystem {
    pub shares: Vec<Share>,
    pub disks: Vec<Disk>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(default)]
pub struct Share {
    pub source: PathBuf,
    pub tag: String,
    pub write: bool,
    pub inode_file_handles: InodeFileHandles,
}

impl Default for Share {
    fn default() -> Self {
        Self {
            source: PathBuf::default(),
            tag: String::default(),
            write: true,
            inode_file_handles: InodeFileHandles::Never,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum InodeFileHandles {
    #[serde(rename = "never")]
    Never,
    #[serde(rename = "prefer")]
    Prefer,
    #[serde(rename = "mandatory")]
    Mandatory,
}

impl Display for InodeFileHandles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Never => "never",
            Self::Prefer => "prefer",
            Self::Mandatory => "mandatory",
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(default)]
pub struct Disk {
    pub source: Option<PathBuf>,
    pub tag: String,
    pub write: bool,
    pub create: bool,
    pub size: u64,
    pub format: Format,
}

impl Default for Disk {
    fn default() -> Self {
        Self {
            source: None,
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

impl Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Qcow2 => "qcow2",
            Self::Raw => "raw",
        })
    }
}
