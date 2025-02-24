use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Memory {
    #[serde(default = "default_size")]
    pub size: u64,
}

fn default_size() -> u64 {
    1024
}
