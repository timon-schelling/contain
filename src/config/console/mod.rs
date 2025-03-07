use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(default)]
pub struct Console {
    pub mode: Mode,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum Mode {
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "log")]
    Log,
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
