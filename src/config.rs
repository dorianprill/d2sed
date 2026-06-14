use crate::model::GameVersion;
use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub export_path: Option<PathBuf>,
    pub selected_version: Option<GameVersion>,
}
