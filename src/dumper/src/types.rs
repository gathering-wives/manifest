use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PakKey {
    pub key: String,
    pub entropy: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetKey {
    pub key: String,
    pub iv: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputFile {
    pub version: String,
    pub pak_keys: Vec<PakKey>,
    pub net_keys: Option<NetKey>,
}
