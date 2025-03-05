use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Cpu {
    pub cores: u64,
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            cores: 1,
        }
    }
}
