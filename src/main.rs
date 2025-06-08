use eframe::egui;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use anyhow::Result;

mod plugin_scanner;
mod plugin_types;

use plugin_scanner::PluginScanner;
use plugin_types::Plugin;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Audio Plugin Cleaner",
        options,
        Box::new(|_cc| Box::new(PluginCleanerApp::new())),
    )
}

struct PluginCleanerApp {
    plugins: BTreeMap<String, Vec<Plugin>>,
    selected_plugins: HashSet<PathBuf>,
    selected_manufacturers: HashSet<String>,
    scanning: bool,
    show_confirmation: bool,
    scanner: PluginScanner,
    deletion_error: Option<String>,
}

impl PluginCleanerApp {
    fn new() -> Self {
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

    fn clean_manufacturer_name(name: &str) -> String {
        static SUFFIX_REGEX: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(?i)[\s,]+(ltd|llc|lcc|inc|gmbh|corp|co|ag|a/s)\.?$").unwrap());
        SUFFIX_REGEX.replace_all(name, "").trim().to_string()
    }

    fn scan_plugins(&mut self) {
        self.scanning = true;
        self.plugins.clear();
        self.selected_plugins.clear();
        self.selected_manufacturers.clear();

        match self.scanner.scan_all_plugins() {
            Ok(plugins) => {
                let mut grouped_by_key: BTreeMap<String, Vec<Plugin>> = BTreeMap::new();
                for plugin in plugins {
                    let cleaned_name = Self::clean_manufacturer_name(&plugin.manufacturer);
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

                    let display_name = Self::clean_manufacturer_name(&most_common_original_name);

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

    fn toggle_manufacturer(&mut self, manufacturer: &str) {
        if self.selected_manufacturers.contains(manufacturer) {
            self.selected_manufacturers.remove(manufacturer);
            if let Some(plugins) = self.plugins.get(manufacturer) {
                for plugin in plugins {
                    self.selected_plugins.remove(&plugin.path);
                }
            }
        } else {
            self.selected_manufacturers.insert(manufacturer.to_string());
            if let Some(plugins) = self.plugins.get(manufacturer) {
                for plugin in plugins {
                    self.selected_plugins.insert(plugin.path.clone());
                }
            }
        }
    }

    fn toggle_plugin(&mut self, plugin: &Plugin) {
        if self.selected_plugins.contains(&plugin.path) {
            self.selected_plugins.remove(&plugin.path);
        } else {
            self.selected_plugins.insert(plugin.path.clone());
        }

        if let Some(plugins) = self.plugins.get(&plugin.manufacturer) {
            let all_selected = plugins.iter().all(|p| self.selected_plugins.contains(&p.path));
            if all_selected {
                self.selected_manufacturers.insert(plugin.manufacturer.clone());
            } else {
                // If not all plugins for this manufacturer are selected,
                // it should not be in the selected_manufacturers set.
                self.selected_manufacturers.remove(&plugin.manufacturer);
            }
        }
    }

    fn delete_selected_plugins(&mut self) {
        if self.selected_plugins.is_empty() {
            return;
        }

        let paths_to_delete: Vec<_> = self.selected_plugins.iter().cloned().collect();
        let mut actually_deleted_paths = HashSet::new();
        let mut deletion_failed = false;

        // Move to trash
        let result = trash::delete_all(&paths_to_delete);
        
        // We must verify which files were actually deleted.
        for path in &paths_to_delete {
            if !path.exists() {
                actually_deleted_paths.insert(path.clone());
            }
        }
        
        // A failure occurred if the operation returned an error OR if not all files were deleted.
        if result.is_err() || actually_deleted_paths.len() < paths_to_delete.len() {
            if let Err(e) = result {
                 eprintln!("Error moving plugins to trash: {}", e);
            }
            deletion_failed = true;
        }

        if deletion_failed {
            self.deletion_error = Some(
                "Some or all plugins could not be moved to bin.\nThis can happen on Windows due to file permissions.\nPlease try running this application as an administrator.".to_string()
            );
        }

        // Update application state based on what was actually deleted.
        if !actually_deleted_paths.is_empty() {
            // Update the set of selected plugins.
            self.selected_plugins.retain(|p| !actually_deleted_paths.contains(p));
            
            // Identify which manufacturers were affected by the deletion.
            let mut affected_manufacturers = HashSet::new();
            self.plugins.retain(|manufacturer, plugins| {
                let original_len = plugins.len();
                plugins.retain(|p| !actually_deleted_paths.contains(&p.path));
                
                if plugins.len() < original_len {
                    affected_manufacturers.insert(manufacturer.clone());
                }
                // Keep the manufacturer in the map only if it still has plugins.
                !plugins.is_empty()
            });

            // If a manufacturer was affected, it means it's no longer fully selected.
            for m_name in affected_manufacturers {
                self.selected_manufacturers.remove(&m_name);
            }
        }
    }
}

impl eframe::App for PluginCleanerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Audio Plugin Cleaner");

            ui.horizontal(|ui| {
                if ui.button("Scan Plugins").clicked() && !self.scanning {
                    self.scan_plugins();
                }

                if self.scanning {
                    ui.spinner();
                    ui.label("Scanning...");
                }

                if !self.selected_plugins.is_empty() {
                    ui.separator();
                    if ui.button("Clear Selection").clicked() {
                        self.selected_plugins.clear();
                        self.selected_manufacturers.clear();
                    }
                    if ui.button(format!("Move to Bin ({})", self.selected_plugins.len())).clicked() {
                        self.show_confirmation = true;
                    }
                }
            });

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let plugins_data: Vec<(String, Vec<Plugin>)> = self.plugins.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                for (manufacturer, plugins) in plugins_data {
                    let mut manufacturer_selected = self.selected_manufacturers.contains(&manufacturer);
                    
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut manufacturer_selected, "").changed() {
                           self.toggle_manufacturer(&manufacturer);
                        }
                        ui.strong(format!("{} ({})", manufacturer, plugins.len()));
                    });

                    ui.indent("plugins", |ui| {
                        for plugin in &plugins {
                            ui.horizontal(|ui| {
                                let mut selected = self.selected_plugins.contains(&plugin.path);
                                if ui.checkbox(&mut selected, "").changed() {
                                    self.toggle_plugin(plugin);
                                }

                                ui.label(&plugin.name);
                                ui.label(format!("({:?})", plugin.plugin_type));
                                
                                if let Some(version) = &plugin.version {
                                    ui.label(format!("v{}", version));
                                }
                            });
                        }
                    });

                    ui.separator();
                }
            });
        });

        if self.show_confirmation {
            egui::Window::new("Confirm Move to Bin")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(format!("Are you sure you want to move these {} plugins to the bin?", self.selected_plugins.len()));
                    
                    ui.separator();
                    
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            let mut sorted_paths: Vec<_> = self.selected_plugins.iter().collect();
                            sorted_paths.sort();
                            for path in sorted_paths {
                                ui.label(path.file_name().unwrap_or_default().to_string_lossy().to_string());
                            }
                        });

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.show_confirmation = false;
                        }

                        if ui.button("Move to Bin").clicked() {
                            self.delete_selected_plugins();
                            self.show_confirmation = false;
                        }
                    });
                });
        }

        // Show a modal error window if a deletion error occurred
        if let Some(error_message) = self.deletion_error.clone() {
            egui::Window::new("Move to Bin Error")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(error_message);
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            self.deletion_error = None;
                        }
                    });
                });
        }
    }
}