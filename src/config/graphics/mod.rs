use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
#[serde(default)]
pub struct Graphics {
    pub virtio_gpu: bool,
}
