fn main() {
    #[cfg(target_os = "windows")]
    {
        embed_resource::compile("src/app.rc", embed_resource::NONE);
    }
    
    println!("cargo:rerun-if-changed=src/app.rc");
    println!("cargo:rerun-if-changed=meta/icon.ico");
    println!("cargo:rerun-if-changed=meta/icon.icns");
}