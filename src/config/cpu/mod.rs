use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct Cpu {
    #[serde(default = "default_cores")]
    pub cores: u64,
}

fn default_cores() -> u64 {
    1
}
