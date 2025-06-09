fn main() {
    #[cfg(target_os = "windows")]
    {
        embed_resource::compile("app.rc", embed_resource::NONE);
    }
    
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/icon.icns");
}