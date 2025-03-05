use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Memory {
    pub size: u64,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            size: 1024,
        }
    }
}
