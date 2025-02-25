use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct Graphics {
    pub virtio_gpu: bool,
}
