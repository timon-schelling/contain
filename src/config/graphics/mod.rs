use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Graphics {
    pub virtio_gpu: bool,
}
