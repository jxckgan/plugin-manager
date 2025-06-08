use crate::plugin_types::{Plugin, PluginType};
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use trash;
use walkdir::WalkDir;

#[cfg(target_os = "macos")]
use plist::Value;

#[cfg(target_os = "windows")]
use std::ffi::OsStr;

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;

#[cfg(target_os = "windows")]
use winapi::ctypes::c_void;

#[cfg(target_os = "windows")]
use winapi::shared::minwindef::UINT;

#[cfg(target_os = "windows")]
use winapi::um::winver::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};

#[cfg(target_os = "windows")]
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

#[cfg(target_os = "windows")]
#[repr(C)]
struct VsFixedFileInfo {
    signature: u32,
    struct_version: u32,
    file_version_ms: u32,
    file_version_ls: u32,
    product_version_ms: u32,
    product_version_ls: u32,
    file_flags_mask: u32,
    file_flags: u32,
    file_os: u32,
    file_type: u32,
    file_subtype: u32,
    file_date_ms: u32,
    file_date_ls: u32,
}

pub struct PluginScanner;

impl PluginScanner {
    pub fn new() -> Self {
        Self {}
    }

    pub fn delete_plugin(&self, plugin: &Plugin) -> Result<()> {
        trash::delete(&plugin.path)
            .with_context(|| format!("Failed to move plugin to trash: {:?}", plugin.path))
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
            for path in self.get_au_paths() {
                if path.exists() {
                    plugins.extend(self.scan_au_directory(&path)?);
                }
            }
        }

        Ok(plugins)
    }

    fn get_vst2_paths(&self) -> Vec<PathBuf> {
        let mut paths = HashSet::new();

        #[cfg(target_os = "windows")]
        {
            if let Some(program_files) = std::env::var_os("ProgramW6432") {
                let program_files_path = PathBuf::from(&program_files);
                paths.insert(program_files_path.join("VstPlugins"));
                paths.insert(program_files_path.join("Steinberg/VstPlugins"));
                paths.insert(program_files_path.join("Common Files/VST2"));
            }
            if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
                let program_files_path = PathBuf::from(&program_files_x86);
                paths.insert(program_files_path.join("VstPlugins"));
                paths.insert(program_files_path.join("Steinberg/VstPlugins"));
                paths.insert(program_files_path.join("Common Files/VST2"));
            }

            paths.insert(PathBuf::from(r"C:\Program Files\Common Files\VST2"));
            paths.insert(PathBuf::from(
                r"C:\Program Files (x86)\Common Files\VST2",
            ));

            if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey("SOFTWARE\\VST") {
                if let Ok(vst_path) = hklm.get_value::<String, _>("VSTPluginsPath") {
                    paths.insert(PathBuf::from(vst_path));
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            paths.insert(PathBuf::from("/Library/Audio/Plug-Ins/VST"));
            if let Some(home) = dirs::home_dir() {
                paths.insert(home.join("Library/Audio/Plug-Ins/VST"));
            }
        }

        paths.into_iter().collect()
    }

    fn get_vst3_paths(&self) -> Vec<PathBuf> {
        let mut paths = HashSet::new();

        #[cfg(target_os = "windows")]
        {
            if let Some(program_files) = std::env::var_os("ProgramW6432") {
                paths.insert(PathBuf::from(program_files).join("Common Files/VST3"));
            }
            if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
                paths.insert(PathBuf::from(program_files_x86).join("Common Files/VST3"));
            }
        }

        #[cfg(target_os = "macos")]
        {
            paths.insert(PathBuf::from("/Library/Audio/Plug-Ins/VST3"));
            if let Some(home) = dirs::home_dir() {
                paths.insert(home.join("Library/Audio/Plug-Ins/VST3"));
            }
        }

        paths.into_iter().collect()
    }

    fn get_aax_paths(&self) -> Vec<PathBuf> {
        let mut paths = HashSet::new();

        #[cfg(target_os = "windows")]
        {
            // Standard AAX plugin paths for Windows
            if let Some(program_files) = std::env::var_os("ProgramW6432") {
                paths.insert(PathBuf::from(program_files).join("Common Files/Avid/Audio/Plug-Ins"));
            }
            if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
                paths.insert(PathBuf::from(program_files_x86).join("Common Files/Avid/Audio/Plug-Ins"));
            }
            
            // Fallback paths
            paths.insert(PathBuf::from(r"C:\Program Files\Common Files\Avid\Audio\Plug-Ins"));
            paths.insert(PathBuf::from(r"C:\Program Files (x86)\Common Files\Avid\Audio\Plug-Ins"));
            
            // Check registry for custom AAX paths
            if let Ok(hklm) = RegKey::predef(HKEY_LOCAL_MACHINE)
                .open_subkey("SOFTWARE\\Avid\\Audio\\Plug-Ins") {
                if let Ok(aax_path) = hklm.get_value::<String, _>("InstallDir") {
                    paths.insert(PathBuf::from(aax_path));
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            // Standard AAX plugin paths for macOS
            paths.insert(PathBuf::from("/Library/Application Support/Avid/Audio/Plug-Ins"));
            
            if let Some(home) = dirs::home_dir() {
                paths.insert(home.join("Library/Application Support/Avid/Audio/Plug-Ins"));
            }
        }

        paths.into_iter().collect()
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

        for entry in WalkDir::new(dir)
            .max_depth(5)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();

            let is_vst2 = if cfg!(target_os = "macos") {
                path.is_dir()
                    && path
                        .extension()
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("vst"))
            } else if cfg!(target_os = "windows") {
                path.is_file() && self.is_potential_vst2_file(path)
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

    #[cfg(target_os = "windows")]
    fn is_potential_vst2_file(&self, path: &Path) -> bool {
        if !path
            .extension()
            .map_or(false, |ext| ext.eq_ignore_ascii_case("dll"))
        {
            return false;
        }

        if std::fs::metadata(path).is_err() {
            return false;
        }

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        let system_dll_patterns = [
            "msvcr",
            "msvcp",
            "vcruntime",
            "api-ms-",
            "kernel32",
            "user32",
            "shell32",
            "ole32",
            "oleaut32",
            "comctl32",
            "comdlg32",
            "gdi32",
            "advapi32",
            "winmm",
            "wsock32",
            "ws2_32",
            "version",
            "shlwapi",
        ];

        if system_dll_patterns
            .iter()
            .any(|pattern| file_name.starts_with(pattern))
        {
            return false;
        }

        true
    }

    fn scan_vst3_directory(&self, dir: &Path) -> Result<Vec<Plugin>> {
        let mut plugins = Vec::new();

        for entry in WalkDir::new(dir)
            .max_depth(5)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();

            let is_vst3 = path
                .extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("vst3"))
                && (path.is_dir() || path.is_file());

            if is_vst3 {
                if let Ok(plugin) = self.parse_vst3_plugin(path) {
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
            .filter_map(Result::ok)
        {
            let path = entry.path();

            let is_aax = if cfg!(target_os = "macos") {
                // macOS AAX plugins are bundles with .aaxplugin extension
                path.is_dir() && path
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("aaxplugin"))
            } else if cfg!(target_os = "windows") {
                // Windows AAX plugins are .aaxplugin files (actually directories)
                // or .aax files in some cases
                (path.is_dir() && path
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("aaxplugin"))) ||
                (path.is_file() && path
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("aax")))
            } else {
                false
            };

            if is_aax {
                if let Ok(plugin) = self.parse_aax_plugin(path) {
                    plugins.push(plugin);
                }
            }
        }

        Ok(plugins)
    }

    #[cfg(target_os = "macos")]
    fn scan_au_directory(&self, dir: &Path) -> Result<Vec<Plugin>> {
        let mut plugins = Vec::new();

        for entry in WalkDir::new(dir)
            .max_depth(5)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();

            if path.is_dir()
                && path
                    .extension()
                    .map_or(false, |ext| ext.eq_ignore_ascii_case("component"))
            {
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

        #[cfg(target_os = "windows")]
        {
            if let Ok((name, manufacturer, version)) = self.parse_windows_dll_metadata(path) {
                return Ok(Plugin {
                    name: name.unwrap_or(default_name),
                    manufacturer: manufacturer.unwrap_or_else(|| "Unknown".to_string()),
                    version,
                    path: path.to_path_buf(),
                    plugin_type: PluginType::VST2,
                });
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
            if path.is_file() {
                if let Ok((name, manufacturer, version)) = self.parse_windows_dll_metadata(path) {
                    return Ok(Plugin {
                        name: name.unwrap_or(default_name),
                        manufacturer: manufacturer.unwrap_or_else(|| "Unknown".to_string()),
                        version,
                        path: path.to_path_buf(),
                        plugin_type: PluginType::VST3,
                    });
                } else {
                    return Ok(Plugin {
                        name: default_name,
                        manufacturer: "Unknown".to_string(),
                        version: None,
                        path: path.to_path_buf(),
                        plugin_type: PluginType::VST3,
                    });
                }
            }

            if path.is_dir() {
                for json_name in ["moduleinfo.json", "plugin.json"] {
                    let json_path = path.join("Contents").join(json_name);
                    if json_path.exists() {
                        if let Ok(file_content) = std::fs::read_to_string(&json_path) {
                            if let Ok(json_value) =
                                serde_json::from_str::<serde_json::Value>(&file_content)
                            {
                                let name = json_value
                                    .get("Name")
                                    .or_else(|| json_value.get("name"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                let manufacturer = json_value
                                    .get("Vendor")
                                    .or_else(|| json_value.get("vendor"))
                                    .or_else(|| json_value.get("Company"))
                                    .or_else(|| json_value.get("company"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                let version = json_value
                                    .get("Version")
                                    .or_else(|| json_value.get("version"))
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());

                                if name.is_some() || manufacturer.is_some() {
                                    return Ok(Plugin {
                                        name: name.unwrap_or_else(|| default_name.clone()),
                                        manufacturer: manufacturer
                                            .unwrap_or_else(|| "Unknown".to_string()),
                                        version,
                                        path: path.to_path_buf(),
                                        plugin_type: PluginType::VST3,
                                    });
                                }
                            }
                        }
                    }
                }

                if let Some(file_name) = path.file_name() {
                    let contents_path = path.join("Contents");
                    let arch_folder = if cfg!(target_arch = "x86_64") {
                        "x86_64-win"
                    } else {
                        "x86-win"
                    };

                    let vst3_executable = contents_path.join(arch_folder).join(file_name);

                    if vst3_executable.exists() {
                        if let Ok((name, manufacturer, version)) =
                            self.parse_windows_dll_metadata(&vst3_executable)
                        {
                            return Ok(Plugin {
                                name: name.unwrap_or(default_name),
                                manufacturer: manufacturer
                                    .unwrap_or_else(|| "Unknown".to_string()),
                                version,
                                path: path.to_path_buf(),
                                plugin_type: PluginType::VST3,
                            });
                        }
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

    fn parse_aax_plugin(&self, path: &Path) -> Result<Plugin> {
        let default_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();

        #[cfg(target_os = "macos")]
        {
            // Try to parse Info.plist for macOS AAX plugins
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
                        plugin_type: PluginType::AAX,
                    });
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            if path.is_dir() {
                // Try to find the actual executable within the .aaxplugin bundle
                if let Some(file_name) = path.file_name() {
                    let contents_path = path.join("Contents");
                    
                    // Check for different architecture folders
                    let arch_folders = if cfg!(target_arch = "x86_64") {
                        vec!["x64", "Win64", "x86_64"]
                    } else {
                        vec!["Win32", "x86"]
                    };

                    for arch_folder in arch_folders {
                        let aax_executable = contents_path.join(arch_folder).join(file_name).with_extension("aaxplugin");
                        if aax_executable.exists() {
                            if let Ok((name, manufacturer, version)) =
                                self.parse_windows_dll_metadata(&aax_executable)
                            {
                                return Ok(Plugin {
                                    name: name.unwrap_or_else(|| default_name.clone()),
                                    manufacturer: manufacturer
                                        .unwrap_or_else(|| "Unknown".to_string()),
                                    version,
                                    path: path.to_path_buf(),
                                    plugin_type: PluginType::AAX,
                                });
                            }
                        }
                    }
                }
                
                // Try to find any .dll or .aax file in the Contents directory
                for entry in WalkDir::new(path.join("Contents"))
                    .max_depth(3)
                    .into_iter()
                    .filter_map(Result::ok)
                {
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        if let Some(ext) = entry_path.extension() {
                            if ext.eq_ignore_ascii_case("dll") || ext.eq_ignore_ascii_case("aax") {
                                if let Ok((name, manufacturer, version)) =
                                    self.parse_windows_dll_metadata(entry_path)
                                {
                                    return Ok(Plugin {
                                        name: name.unwrap_or_else(|| default_name.clone()),
                                        manufacturer: manufacturer
                                            .unwrap_or_else(|| "Unknown".to_string()),
                                        version,
                                        path: path.to_path_buf(),
                                        plugin_type: PluginType::AAX,
                                    });
                                }
                            }
                        }
                    }
                }
            } else if path.is_file() {
                // Direct .aax file
                if let Ok((name, manufacturer, version)) = self.parse_windows_dll_metadata(path) {
                    return Ok(Plugin {
                        name: name.unwrap_or(default_name),
                        manufacturer: manufacturer.unwrap_or_else(|| "Unknown".to_string()),
                        version,
                        path: path.to_path_buf(),
                        plugin_type: PluginType::AAX,
                    });
                }
            }
        }

        // Fallback for when metadata parsing fails
        Ok(Plugin {
            name: default_name,
            manufacturer: "Unknown".to_string(),
            version: None,
            path: path.to_path_buf(),
            plugin_type: PluginType::AAX,
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

    #[cfg(target_os = "windows")]
    fn parse_windows_dll_metadata(
        &self,
        path: &Path,
    ) -> Result<(Option<String>, Option<String>, Option<String>)> {
        let path_wide: Vec<u16> = OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let size = GetFileVersionInfoSizeW(path_wide.as_ptr(), std::ptr::null_mut());
            if size == 0 {
                return Err(anyhow::anyhow!(
                    "GetFileVersionInfoSizeW failed for {:?}",
                    path
                ));
            }

            let mut buffer = vec![0u8; size as usize];
            if GetFileVersionInfoW(
                path_wide.as_ptr(),
                0,
                size,
                buffer.as_mut_ptr() as *mut _,
            ) == 0
            {
                return Err(anyhow::anyhow!("GetFileVersionInfoW failed for {:?}", path));
            }

            let mut version_info: *mut c_void = std::ptr::null_mut();
            let mut version_len: UINT = 0;
            let version_query: Vec<u16> = OsStr::new("\\")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let version = if VerQueryValueW(
                buffer.as_ptr() as *const _,
                version_query.as_ptr(),
                &mut version_info,
                &mut version_len,
            ) != 0
                && !version_info.is_null()
            {
                let info = &*(version_info as *const VsFixedFileInfo);
                let major = (info.file_version_ms >> 16) & 0xFFFF;
                let minor = info.file_version_ms & 0xFFFF;
                let build = (info.file_version_ls >> 16) & 0xFFFF;
                let revision = info.file_version_ls & 0xFFFF;
                Some(format!("{}.{}.{}.{}", major, minor, build, revision))
            } else {
                None
            };

            let lang_codepages = self.get_language_codepages(&buffer)?;

            let get_value = |key: &str| -> Option<String> {
                lang_codepages
                    .iter()
                    .find_map(|lcp| self.get_string_file_info(&buffer, lcp, key).ok().flatten())
            };

            let product_name = get_value("ProductName");
            let company_name = get_value("CompanyName");
            let file_description = get_value("FileDescription");

            let name = product_name.or(file_description);

            Ok((name, company_name, version))
        }
    }

    #[cfg(target_os = "windows")]
    fn get_language_codepages(&self, buffer: &[u8]) -> Result<Vec<String>> {
        unsafe {
            let mut translation: *mut c_void = std::ptr::null_mut();
            let mut trans_len: UINT = 0;
            let trans_query: Vec<u16> = OsStr::new("\\VarFileInfo\\Translation")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            if VerQueryValueW(
                buffer.as_ptr() as *const _,
                trans_query.as_ptr(),
                &mut translation,
                &mut trans_len,
            ) != 0
                && !translation.is_null()
                && trans_len > 0
            {
                let num_translations = trans_len as usize / std::mem::size_of::<u32>();
                let translation_ptr = translation as *const u16;
                let mut codepages = Vec::with_capacity(num_translations);
                for i in 0..num_translations {
                    let lang_id = *translation_ptr.add(i * 2);
                    let codepage = *translation_ptr.add(i * 2 + 1);
                    codepages.push(format!("{:04X}{:04X}", lang_id, codepage));
                }
                return Ok(codepages);
            }
        }
        Ok(vec!["040904B0".to_string()])
    }

    #[cfg(target_os = "windows")]
    fn get_string_file_info(
        &self,
        buffer: &[u8],
        lang_codepage: &str,
        key: &str,
    ) -> Result<Option<String>> {
        let query = format!("\\StringFileInfo\\{}\\{}", lang_codepage, key);
        let query_wide: Vec<u16> = OsStr::new(&query)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut value: *mut c_void = std::ptr::null_mut();
        let mut value_len: UINT = 0;

        unsafe {
            if VerQueryValueW(
                buffer.as_ptr() as *const _,
                query_wide.as_ptr(),
                &mut value,
                &mut value_len,
            ) != 0
                && !value.is_null()
                && value_len > 0
            {
                let value_ptr = value as *const u16;
                let slice = std::slice::from_raw_parts(value_ptr, (value_len - 1) as usize);

                let string_value = String::from_utf16(slice).unwrap_or_default();
                if !string_value.trim().is_empty() {
                    return Ok(Some(string_value.trim().to_string()));
                }
            }
        }

        Ok(None)
    }
}