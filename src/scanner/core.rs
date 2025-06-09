use crate::plugin::Plugin;
use crate::utils::error::Result;
use std::path::Path;
use walkdir::WalkDir;

use super::metadata::{parse_aax_plugin, parse_vst2_plugin, parse_vst3_plugin};

#[cfg(target_os = "macos")]
use super::macos::scan_au_directory;

pub struct PluginScanner;

impl PluginScanner {
    pub fn new() -> Self {
        Self {}
    }

    pub fn scan_all_plugins(&self) -> Result<Vec<Plugin>> {
        let mut plugins = Vec::new();

        for path in self.get_vst2_paths() {
            if path.exists() {
                plugins.extend(self.scan_vst2_directory(&path)?);
            }
        }

        for path in self.get_vst3_paths() {
            if path.exists() {
                plugins.extend(self.scan_vst3_directory(&path)?);
            }
        }

        for path in self.get_aax_paths() {
            if path.exists() {
                plugins.extend(self.scan_aax_directory(&path)?);
            }
        }

        #[cfg(target_os = "macos")]
        {
            for path in super::macos::get_au_paths() {
                if path.exists() {
                    plugins.extend(scan_au_directory(&path)?);
                }
            }
        }

        Ok(plugins)
    }

    #[cfg(target_os = "windows")]
    fn get_vst2_paths(&self) -> Vec<std::path::PathBuf> {
        super::windows::get_vst2_paths()
    }

    #[cfg(target_os = "macos")]
    fn get_vst2_paths(&self) -> Vec<std::path::PathBuf> {
        super::macos::get_vst2_paths()
    }

    #[cfg(target_os = "windows")]
    fn get_vst3_paths(&self) -> Vec<std::path::PathBuf> {
        super::windows::get_vst3_paths()
    }

    #[cfg(target_os = "macos")]
    fn get_vst3_paths(&self) -> Vec<std::path::PathBuf> {
        super::macos::get_vst3_paths()
    }

    #[cfg(target_os = "windows")]
    fn get_aax_paths(&self) -> Vec<std::path::PathBuf> {
        super::windows::get_aax_paths()
    }

    #[cfg(target_os = "macos")]
    fn get_aax_paths(&self) -> Vec<std::path::PathBuf> {
        super::macos::get_aax_paths()
    }

    fn scan_vst2_directory(&self, dir: &Path) -> Result<Vec<Plugin>> {
        let mut plugins = Vec::new();

        for entry in WalkDir::new(dir)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            #[cfg(target_os = "macos")]
            let is_vst2 = path.is_dir()
                && path
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("vst"));

            #[cfg(target_os = "windows")]
            let is_vst2 = path.is_file() && super::windows::is_potential_vst2_file(path);

            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            let is_vst2 = false;

            if is_vst2 {
                if let Ok(plugin) = parse_vst2_plugin(path) {
                    plugins.push(plugin);
                }
            }
        }

        Ok(plugins)
    }

    fn scan_vst3_directory(&self, dir: &Path) -> Result<Vec<Plugin>> {
        let mut plugins = Vec::new();

        for entry in WalkDir::new(dir)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            let is_vst3 = path
                .extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("vst3"))
                && (path.is_dir() || path.is_file());

            if is_vst3 {
                if let Ok(plugin) = parse_vst3_plugin(path) {
                    plugins.push(plugin);
                }
            }
        }

        Ok(plugins)
    }

    fn scan_aax_directory(&self, dir: &Path) -> Result<Vec<Plugin>> {
        let mut plugins = Vec::new();

        for entry in WalkDir::new(dir)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            let is_aax = if cfg!(target_os = "macos") {
                path.is_dir()
                    && path
                        .extension()
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("aaxplugin"))
            } else if cfg!(target_os = "windows") {
                (path.is_dir()
                    && path
                        .extension()
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("aaxplugin")))
                    || (path.is_file()
                        && path
                            .extension()
                            .map_or(false, |ext| ext.eq_ignore_ascii_case("aax")))
            } else {
                false
            };

            if is_aax {
                if let Ok(plugin) = parse_aax_plugin(path) {
                    plugins.push(plugin);
                }
            }
        }

        Ok(plugins)
    }
}
