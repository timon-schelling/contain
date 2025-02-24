use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct NetTapCreateRequest {
    pub user: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetTapCreateResponse {
    pub name: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetTapDeleteRequest {
    pub name: String
}
