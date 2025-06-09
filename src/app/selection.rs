use super::state::PluginManager;
use crate::plugin::Plugin;
use std::collections::HashSet;

impl PluginManager {
    pub fn toggle_manufacturer(&mut self, manufacturer: &str) {
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

    pub fn toggle_plugin(&mut self, plugin: &Plugin) {
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
                self.selected_manufacturers.remove(&plugin.manufacturer);
            }
        }
    }

    pub fn delete_selected_plugins(&mut self) {
        if self.selected_plugins.is_empty() {
            return;
        }

        let paths_to_delete: Vec<_> = self.selected_plugins.iter().cloned().collect();
        let mut actually_deleted_paths = HashSet::new();
        let mut deletion_failed = false;

        let result = trash::delete_all(&paths_to_delete);

        for path in &paths_to_delete {
            if !path.exists() {
                actually_deleted_paths.insert(path.clone());
            }
        }

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

        if !actually_deleted_paths.is_empty() {
            self.selected_plugins.retain(|p| !actually_deleted_paths.contains(p));

            let mut affected_manufacturers = HashSet::new();
            self.plugins.retain(|manufacturer, plugins| {
                let original_len = plugins.len();
                plugins.retain(|p| !actually_deleted_paths.contains(&p.path));

                if plugins.len() < original_len {
                    affected_manufacturers.insert(manufacturer.clone());
                }
                !plugins.is_empty()
            });

            for m_name in affected_manufacturers {
                self.selected_manufacturers.remove(&m_name);
            }
        }
    }
}
