use crate::plugin::Plugin;
use crate::utils::error::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::metadata::parse_au_plugin;

pub(super) fn get_vst2_paths() -> Vec<PathBuf> {
    let mut paths = HashSet::new();
    paths.insert(PathBuf::from("/Library/Audio/Plug-Ins/VST"));
    if let Some(home) = dirs::home_dir() {
        paths.insert(home.join("Library/Audio/Plug-Ins/VST"));
    }
    paths.into_iter().collect()
}

pub(super) fn get_vst3_paths() -> Vec<PathBuf> {
    let mut paths = HashSet::new();
    paths.insert(PathBuf::from("/Library/Audio/Plug-Ins/VST3"));
    if let Some(home) = dirs::home_dir() {
        paths.insert(home.join("Library/Audio/Plug-Ins/VST3"));
    }
    paths.into_iter().collect()
}

pub(super) fn get_aax_paths() -> Vec<PathBuf> {
    let mut paths = HashSet::new();
    paths.insert(PathBuf::from(
        "/Library/Application Support/Avid/Audio/Plug-Ins",
    ));
    if let Some(home) = dirs::home_dir() {
        paths.insert(home.join("Library/Application Support/Avid/Audio/Plug-Ins"));
    }
    paths.into_iter().collect()
}

pub(super) fn get_au_paths() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from("/Library/Audio/Plug-Ins/Components")];
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join("Library/Audio/Plug-Ins/Components"));
    }
    paths
}

pub(super) fn scan_au_directory(dir: &Path) -> Result<Vec<Plugin>> {
    let mut plugins = Vec::new();
    for entry in WalkDir::new(dir)
        .max_depth(5)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_dir()
            && path
                .extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("component"))
        {
            if let Ok(plugin) = parse_au_plugin(path) {
                plugins.push(plugin);
            }
        }
    }
    Ok(plugins)
}
