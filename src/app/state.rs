use crate::plugin::{clean_manufacturer_name, Plugin};
use crate::scanner::PluginScanner;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

pub struct PluginManager {
    pub plugins: BTreeMap<String, Vec<Plugin>>,
    pub selected_plugins: HashSet<PathBuf>,
    pub selected_manufacturers: HashSet<String>,
    pub scanning: bool,
    pub show_confirmation: bool,
    pub scanner: PluginScanner,
    pub deletion_error: Option<String>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: BTreeMap::new(),
            selected_plugins: HashSet::new(),
            selected_manufacturers: HashSet::new(),
            scanning: false,
            show_confirmation: false,
            scanner: PluginScanner::new(),
            deletion_error: None,
        }
    }

    pub fn scan_plugins(&mut self) {
        self.scanning = true;
        self.plugins.clear();
        self.selected_plugins.clear();
        self.selected_manufacturers.clear();
        self.deletion_error = None;

        match self.scanner.scan_all_plugins() {
            Ok(plugins) => {
                let mut grouped_by_key: BTreeMap<String, Vec<Plugin>> = BTreeMap::new();
                for plugin in plugins {
                    let cleaned_name = clean_manufacturer_name(&plugin.manufacturer);
                    let key = cleaned_name.to_lowercase().replace(['-', ' '], "");
                    grouped_by_key.entry(key).or_default().push(plugin);
                }

                let mut final_plugins: BTreeMap<String, Vec<Plugin>> = BTreeMap::new();
                for (_key, mut group) in grouped_by_key {
                    let mut counts = BTreeMap::new();
                    for p in &group {
                        *counts.entry(p.manufacturer.as_str()).or_insert(0) += 1;
                    }

                    let most_common_original_name = counts
                        .into_iter()
                        .max_by(|a, b| {
                            a.1.cmp(&b.1)
                                .then_with(|| b.0.contains('-').cmp(&a.0.contains('-')))
                                .then_with(|| a.0.len().cmp(&b.0.len()))
                        })
                        .map(|(name, _)| name.to_string())
                        .unwrap_or_else(|| {
                            group
                                .first()
                                .map(|p| p.manufacturer.clone())
                                .unwrap_or_else(|| "Unknown".to_string())
                        });

                    let display_name = clean_manufacturer_name(&most_common_original_name);

                    for p in &mut group {
                        p.manufacturer = display_name.clone();
                    }

                    group.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

                    final_plugins.insert(display_name, group);
                }
                self.plugins = final_plugins;
            }
            Err(e) => {
                eprintln!("Error scanning plugins: {}", e);
            }
        }

        self.scanning = false;
    }
}
