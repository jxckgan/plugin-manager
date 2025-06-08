use crate::plugin_types::{Plugin, PluginType};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[cfg(target_os = "macos")]
use plist::Value;

#[cfg(target_os = "windows")]
use serde::Deserialize;

#[cfg(target_os = "windows")]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct Vst3PluginInfo {
    name: String,
    vendor: String,
    version: String,
}

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

        #[cfg(target_os = "macos")]
        {
            for path in self.get_au_paths() {
                if path.exists() {
                    plugins.extend(self.scan_au_directory(&path)?);
                }
            }
        }

        Ok(plugins)
    }

    fn get_vst2_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        #[cfg(target_os = "windows")]
        {
            paths.push(PathBuf::from(r"C:\Program Files\VstPlugins"));
            paths.push(PathBuf::from(r"C:\Program Files (x86)\VstPlugins"));
            paths.push(PathBuf::from(r"C:\Program Files\Steinberg\VstPlugins"));
            paths.push(PathBuf::from(r"C:\Program Files (x86)\Steinberg\VstPlugins"));
            if let Some(program_files) = std::env::var_os("ProgramW6432") {
                paths.push(PathBuf::from(program_files).join("VstPlugins"));
                paths.push(PathBuf::from(program_files).join("Steinberg/VstPlugins"));
                paths.push(PathBuf::from(program_files).join("Common Files/VST2"));
            }
        }

        #[cfg(target_os = "macos")]
        {
            paths.push(PathBuf::from("/Library/Audio/Plug-Ins/VST"));
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join("Library/Audio/Plug-Ins/VST"));
            }
        }

        paths
    }

    fn get_vst3_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        #[cfg(target_os = "windows")]
        {
            paths.push(PathBuf::from(r"C:\Program Files\Common Files\VST3"));
            if let Some(program_files) = std::env::var_os("ProgramW6432") {
                paths.push(PathBuf::from(program_files).join("Common Files/VST3"));
            }
        }

        #[cfg(target_os = "macos")]
        {
            paths.push(PathBuf::from("/Library/Audio/Plug-Ins/VST3"));
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join("Library/Audio/Plug-Ins/VST3"));
            }
        }

        paths
    }

    #[cfg(target_os = "macos")]
    fn get_au_paths(&self) -> Vec<PathBuf> {
        let mut paths = vec![PathBuf::from("/Library/Audio/Plug-Ins/Components")];
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Audio/Plug-Ins/Components"));
        }
        paths
    }

    fn scan_vst2_directory(&self, dir: &Path) -> Result<Vec<Plugin>> {
        let mut plugins = Vec::new();

        for entry in WalkDir::new(dir).max_depth(3).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            
            let is_vst2 = if cfg!(target_os = "macos") {
                path.is_dir() && path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("vst"))
            } else if cfg!(target_os = "windows") {
                path.is_file() && path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("dll"))
            } else {
                false
            };
            
            if is_vst2 {
                if let Ok(plugin) = self.parse_vst2_plugin(path) {
                    plugins.push(plugin);
                }
            }
        }

        Ok(plugins)
    }

    fn scan_vst3_directory(&self, dir: &Path) -> Result<Vec<Plugin>> {
        let mut plugins = Vec::new();

        for entry in WalkDir::new(dir).max_depth(3).into_iter().filter_map(Result::ok) {
            let path = entry.path();

            if path.is_dir() && path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("vst3")) {
                if let Ok(plugin) = self.parse_vst3_plugin(path) {
                    plugins.push(plugin);
                }
            }
        }

        Ok(plugins)
    }

    #[cfg(target_os = "macos")]
    fn scan_au_directory(&self, dir: &Path) -> Result<Vec<Plugin>> {
        let mut plugins = Vec::new();

        for entry in WalkDir::new(dir).max_depth(3).into_iter().filter_map(Result::ok) {
            let path = entry.path();

            if path.is_dir() && path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("component")) {
                if let Ok(plugin) = self.parse_au_plugin(path) {
                    plugins.push(plugin);
                }
            }
        }

        Ok(plugins)
    }

    fn parse_vst2_plugin(&self, path: &Path) -> Result<Plugin> {
        let default_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        #[cfg(target_os = "macos")]
        {
            let info_plist_path = path.join("Contents/Info.plist");
            if info_plist_path.exists() {
                if let Ok((parsed_name, parsed_manufacturer, version)) =
                    self.parse_info_plist(&info_plist_path)
                {
                    return Ok(Plugin {
                        name: parsed_name.unwrap_or(default_name),
                        manufacturer: parsed_manufacturer.unwrap_or_else(|| "Unknown".to_string()),
                        version,
                        path: path.to_path_buf(),
                        plugin_type: PluginType::VST2,
                    });
                }
            }
        }

        Ok(Plugin {
            name: default_name,
            manufacturer: "Unknown".to_string(),
            version: None,
            path: path.to_path_buf(),
            plugin_type: PluginType::VST2,
        })
    }

    fn parse_vst3_plugin(&self, path: &Path) -> Result<Plugin> {
        let default_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        #[cfg(target_os = "macos")]
        {
            let info_plist_path = path.join("Contents/Info.plist");
            if info_plist_path.exists() {
                if let Ok((parsed_name, parsed_manufacturer, version)) =
                    self.parse_info_plist(&info_plist_path)
                {
                    return Ok(Plugin {
                        name: parsed_name.unwrap_or(default_name),
                        manufacturer: parsed_manufacturer.unwrap_or_else(|| "Unknown".to_string()),
                        version,
                        path: path.to_path_buf(),
                        plugin_type: PluginType::VST3,
                    });
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            let plugin_json_path = path.join("Contents").join("Resources").join("plugin.json");
            if plugin_json_path.exists() {
                if let Ok(file_content) = std::fs::read_to_string(&plugin_json_path) {
                    if let Ok(info) = serde_json::from_str::<Vst3PluginInfo>(&file_content) {
                        return Ok(Plugin {
                            name: info.name,
                            manufacturer: info.vendor,
                            version: Some(info.version),
                            path: path.to_path_buf(),
                            plugin_type: PluginType::VST3,
                        });
                    }
                }
            }
        }
        
        Ok(Plugin {
            name: default_name,
            manufacturer: "Unknown".to_string(),
            version: None,
            path: path.to_path_buf(),
            plugin_type: PluginType::VST3,
        })
    }

    #[cfg(target_os = "macos")]
    fn parse_au_plugin(&self, path: &Path) -> Result<Plugin> {
        let default_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        let info_plist_path = path.join("Contents/Info.plist");
        if info_plist_path.exists() {
            if let Ok((parsed_name, parsed_manufacturer, version)) =
                self.parse_info_plist(&info_plist_path)
            {
                return Ok(Plugin {
                    name: parsed_name.unwrap_or(default_name),
                    manufacturer: parsed_manufacturer.unwrap_or_else(|| "Unknown".to_string()),
                    version,
                    path: path.to_path_buf(),
                    plugin_type: PluginType::AU,
                });
            }
        }

        Ok(Plugin {
            name: default_name,
            manufacturer: "Unknown".to_string(),
            version: None,
            path: path.to_path_buf(),
            plugin_type: PluginType::AU,
        })
    }

    #[cfg(target_os = "macos")]
    fn parse_info_plist(
        &self,
        plist_path: &Path,
    ) -> Result<(Option<String>, Option<String>, Option<String>)> {
        let plist_data = std::fs::read(plist_path)?;
        let plist: Value =
            plist::from_bytes(&plist_data).context("Failed to parse plist from bytes")?;
        let root_dict = plist
            .as_dictionary()
            .context("Plist root is not a dictionary")?;

        let mut name: Option<String> = None;
        let mut manufacturer: Option<String> = None;

        if let Some(components) = root_dict.get("AudioComponents").and_then(Value::as_array) {
            if let Some(component_dict) = components.get(0).and_then(Value::as_dictionary) {
                if let Some(full_name) = component_dict.get("name").and_then(Value::as_string) {
                    if let Some((manuf, plug_name)) = full_name.split_once(':') {
                        manufacturer = Some(manuf.trim().to_string());
                        name = Some(plug_name.trim().to_string());
                    }
                }
            }
        }

        if name.is_none() {
            name = root_dict
                .get("CFBundleName")
                .or_else(|| root_dict.get("CFBundleDisplayName"))
                .and_then(Value::as_string)
                .map(str::to_string);
        }

        if manufacturer.is_none() {
            manufacturer = root_dict
                .get("CFBundleIdentifier")
                .and_then(Value::as_string)
                .and_then(|id| {
                    let parts: Vec<&str> = id.split('.').collect();
                    if parts.len() >= 2 {
                        Some(parts[1].to_string())
                    } else {
                        None
                    }
                });
        }

        let version = root_dict
            .get("CFBundleShortVersionString")
            .or_else(|| root_dict.get("CFBundleVersion"))
            .and_then(Value::as_string)
            .map(str::to_string);

        Ok((name, manufacturer, version))
    }
}