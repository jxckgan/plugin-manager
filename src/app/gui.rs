use super::state::PluginManager;
use crate::plugin::Plugin;
use eframe::egui;

impl eframe::App for PluginManager {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Plugin Manager");

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
                    if ui
                        .button(format!("Move to Bin ({})", self.selected_plugins.len()))
                        .clicked()
                    {
                        self.show_confirmation = true;
                    }
                }
            });

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                let plugins_data: Vec<(String, Vec<Plugin>)> = self
                    .plugins
                    .iter()
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
                    ui.label(format!(
                        "Are you sure you want to move these {} plugins to the bin?",
                        self.selected_plugins.len()
                    ));

                    ui.separator();

                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        let mut sorted_paths: Vec<_> = self.selected_plugins.iter().collect();
                        sorted_paths.sort();
                        for path in sorted_paths {
                            ui.label(
                                path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                            );
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
