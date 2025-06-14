use crate::plugin::{Plugin, PluginType};
use crate::utils::error::Result;
use std::path::Path;

#[cfg(target_os = "windows")]
use walkdir::WalkDir;

#[cfg(target_os = "macos")]
use plist::Value;
#[cfg(target_os = "macos")]
use anyhow::Context;

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

pub(super) fn parse_vst2_plugin(path: &Path) -> Result<Plugin> {
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
                parse_info_plist(&info_plist_path)
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
        if let Ok((name, manufacturer, version)) = parse_windows_dll_metadata(path) {
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

pub(super) fn parse_vst3_plugin(path: &Path) -> Result<Plugin> {
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
                parse_info_plist(&info_plist_path)
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
            if let Ok((name, manufacturer, version)) = parse_windows_dll_metadata(path) {
                return Ok(Plugin {
                    name: name.unwrap_or(default_name),
                    manufacturer: manufacturer.unwrap_or_else(|| "Unknown".to_string()),
                    version,
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

            let contents_path = path.join("Contents");
            if contents_path.is_dir() {
                for entry in WalkDir::new(&contents_path)
                    .max_depth(3)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        if let Some(ext) = entry_path.extension() {
                            if ext.eq_ignore_ascii_case("vst3")
                                || ext.eq_ignore_ascii_case("dll")
                            {
                                if let Ok((name, manufacturer, version)) =
                                    parse_windows_dll_metadata(entry_path)
                                {
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

pub(super) fn parse_aax_plugin(path: &Path) -> Result<Plugin> {
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
                parse_info_plist(&info_plist_path)
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
            let contents_path = path.join("Contents");
            if contents_path.is_dir() {
                for entry in WalkDir::new(&contents_path)
                    .max_depth(3)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let entry_path = entry.path();
                    if entry_path.is_file() {
                        if let Some(ext) = entry_path.extension() {
                            if ext.eq_ignore_ascii_case("aaxplugin")
                                || ext.eq_ignore_ascii_case("aax")
                                || ext.eq_ignore_ascii_case("dll")
                            {
                                if let Ok((name, manufacturer, version)) =
                                    parse_windows_dll_metadata(entry_path)
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
            }
        } else if path.is_file() {
            if let Ok((name, manufacturer, version)) = parse_windows_dll_metadata(path) {
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

    Ok(Plugin {
        name: default_name,
        manufacturer: "Unknown".to_string(),
        version: None,
        path: path.to_path_buf(),
        plugin_type: PluginType::AAX,
    })
}

#[cfg(target_os = "macos")]
pub(super) fn parse_au_plugin(path: &Path) -> Result<Plugin> {
    let default_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unknown")
        .to_string();

    let info_plist_path = path.join("Contents/Info.plist");
    if info_plist_path.exists() {
        if let Ok((parsed_name, parsed_manufacturer, version)) = parse_info_plist(&info_plist_path)
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

        let lang_codepages = get_language_codepages(&buffer)?;

        let get_value = |key: &str| -> Option<String> {
            lang_codepages
                .iter()
                .find_map(|lcp| get_string_file_info(&buffer, lcp, key).ok().flatten())
        };

        let product_name = get_value("ProductName");
        let company_name = get_value("CompanyName");
        let file_description = get_value("FileDescription");

        let name = product_name.or(file_description);

        Ok((name, company_name, version))
    }
}

#[cfg(target_os = "windows")]
fn get_language_codepages(buffer: &[u8]) -> Result<Vec<String>> {
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
