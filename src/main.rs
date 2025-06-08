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
    move_to_trash: bool,
    scanner: PluginScanner,
}

impl PluginCleanerApp {
    fn new() -> Self {
        Self {
            plugins: BTreeMap::new(),
            selected_plugins: HashSet::new(),
            selected_manufacturers: HashSet::new(),
            scanning: false,
            show_confirmation: false,
            move_to_trash: true,
            scanner: PluginScanner::new(),
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
                let none_selected = plugins.iter().all(|p| !self.selected_plugins.contains(&p.path));
                if none_selected {
                   self.selected_manufacturers.remove(&plugin.manufacturer);
                }
            }
        }
    }


    fn delete_selected_plugins(&mut self) -> Result<()> {
        if !self.selected_plugins.is_empty() {
            if self.move_to_trash {
                trash::delete_all(&self.selected_plugins)?;
            } else {
                for path in &self.selected_plugins {
                    if path.is_file() {
                        std::fs::remove_file(path)?;
                    } else if path.is_dir() {
                        std::fs::remove_dir_all(path)?;
                    }
                }
            }
        }

        self.plugins.retain(|_, plugins| {
            plugins.retain(|p| !self.selected_plugins.contains(&p.path));
            !plugins.is_empty()
        });

        self.selected_plugins.clear();
        self.selected_manufacturers.clear();
        Ok(())
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

                ui.separator();

                ui.radio_value(&mut self.move_to_trash, true, "Move to Bin");
                ui.radio_value(&mut self.move_to_trash, false, "Delete Permanently");

                if !self.selected_plugins.is_empty() {
                    let action = if self.move_to_trash { "Move to Bin" } else { "Delete" };
                    if ui.button(format!("{} Selected ({})", action, self.selected_plugins.len())).clicked() {
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
                    
                    let response = ui.horizontal(|ui| {
                        if ui.checkbox(&mut manufacturer_selected, "").changed() {
                           self.toggle_manufacturer(&manufacturer);
                        }
                        ui.strong(format!("{} ({})", manufacturer, plugins.len()));
                    });

                    if response.response.clicked() {
                    }

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
            egui::Window::new("Confirm Deletion")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    let action = if self.move_to_trash { "move to bin" } else { "permanently delete" };
                    ui.label(format!("Are you sure you want to {} these {} plugins?", action, self.selected_plugins.len()));
                    
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

                        let button_text = if self.move_to_trash { "Move to Bin" } else { "Delete" };
                        if ui.button(button_text).clicked() {
                            if let Err(e) = self.delete_selected_plugins() {
                                eprintln!("Error deleting plugins: {}", e);
                            }
                            self.show_confirmation = false;
                        }
                    });
                });
        }
    }
}