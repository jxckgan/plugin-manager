# plugin-manager

`plugin-manager` is an application that scans (AU, AAX, VST2/3) audio plugins and groups them by their vendors. It allows you to bulk delete (via moving to the trash) or manually select plugins by said vendors.

<img width="30%" alt="plugin manager screenshot" src="https://github.com/user-attachments/assets/78edacd2-f79c-4941-b088-3acbd4afcfee" />

> [!NOTE]
> When deleting a large amount of plugins, the app may look as though it's frozen/crashed - it hasn't. 

### Build Notes

Build like any other Rust app; on macOS you can make an application bundle by running `cargo bundle --release`. For Windows, just run `cargo build --release`.

### To-do

- [ ] Add CLAP support
- [ ] Add ability to move plugins
- [ ] Fix group naming bug on macOS (doesn't impact ownership grouping, just an aesthetics issue)
- [ ] Progress-indicator for moving to trash so it doesn't look as if it's crashed with large amounts of plugins

> Made because Plugin Alliance doesn't have a bloody uninstaller...