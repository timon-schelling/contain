use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct Graphics {
    pub virtio_gpu: bool,
}
