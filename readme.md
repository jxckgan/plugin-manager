# plugin-manager

`plugin-manager` is an application, written in Rust, for scanning (AU, AAX, VST2, VST3) audio plugins and grouping them by their vendors. It allows you to bulk delete (move to trash) or manually select plugins by vendor.

> Temporary bug: All items will be grouped properly (though always double check what you're) but sometimes the name won't be exact (e.g. Roland becomes "co")

### To-do

- [ ] Add CLAP support
- [ ] Add ability to move plugins
- [ ] Fix that bug

> Made because Plugin Alliance doesn't have a bloody uninstaller...