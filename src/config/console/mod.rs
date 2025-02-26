use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct Console {
    pub mode: Mode,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Mode {
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "on")]
    On,
    #[serde(rename = "serial")]
    Serial,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Off
    }
}