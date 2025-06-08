# plugin-manager

`plugin-manager` is an application, written in Rust, for scanning (AU, AAX, VST2, VST3) audio plugins and grouping them by their vendors. It allows you to bulk delete by vendor, or to manually select which plugins. You can also select which plugin type you want to list (all will be grouped by default).

> Temporary bug: All items will be grouped properly (though double check what you're selecting) but sometimes the name won't be exact (e.g. Roland becomes "co")

### To-do

- [ ] Add CLAP
- [ ] Add ability to move plugins
- [ ] Fix that bug

> Made because Plugin Alliance doesn't have a bloody uninstaller...