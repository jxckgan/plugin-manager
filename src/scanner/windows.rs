#![cfg(target_os = "windows")]
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

pub(super) fn get_vst2_paths() -> Vec<PathBuf> {
    let mut paths = HashSet::new();
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
    paths.into_iter().collect()
}

pub(super) fn get_vst3_paths() -> Vec<PathBuf> {
    let mut paths = HashSet::new();
    if let Some(program_files) = std::env::var_os("ProgramW6432") {
        paths.insert(PathBuf::from(program_files).join("Common Files/VST3"));
    }
    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        paths.insert(PathBuf::from(program_files_x86).join("Common Files/VST3"));
    }
    paths.into_iter().collect()
}

pub(super) fn get_aax_paths() -> Vec<PathBuf> {
    let mut paths = HashSet::new();
    if let Some(program_files) = std::env::var_os("ProgramW6432") {
        paths.insert(PathBuf::from(program_files).join("Common Files/Avid/Audio/Plug-Ins"));
    }
    if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
        paths.insert(PathBuf::from(program_files_x86).join("Common Files/Avid/Audio/Plug-Ins"));
    }

    paths.insert(PathBuf::from(
        r"C:\Program Files\Common Files\Avid\Audio\Plug-Ins",
    ));
    paths.insert(PathBuf::from(
        r"C:\Program Files (x86)\Common Files\Avid\Audio\Plug-Ins",
    ));

    if let Ok(hklm) =
        RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey("SOFTWARE\\Avid\\Audio\\Plug-Ins")
    {
        if let Ok(aax_path) = hklm.get_value::<String, _>("InstallDir") {
            paths.insert(PathBuf::from(aax_path));
        }
    }
    paths.into_iter().collect()
}

pub(super) fn is_potential_vst2_file(path: &Path) -> bool {
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
