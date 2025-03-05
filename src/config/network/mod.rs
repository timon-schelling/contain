use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Network {
    pub assign_tap_device: bool,
}
