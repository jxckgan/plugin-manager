use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginType {
    VST2,
    VST3,
    AU,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub name: String,
    pub manufacturer: String,
    pub version: Option<String>,
    pub path: PathBuf,
    pub plugin_type: PluginType,
}