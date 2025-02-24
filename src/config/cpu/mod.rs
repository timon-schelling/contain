use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Cpu {
    #[serde(default = "default_cores")]
    pub cores: u64,
}

fn default_cores() -> u64 {
    1
}
