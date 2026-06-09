fn main() {
    slint_build::compile("ui/menu.slint").unwrap();
    
    #[cfg(target_os = "linux")]
    {
        // Copy desktop file to target
        println!("cargo:rerun-if-changed=maccy-kde.desktop");
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let src = std::path::Path::new("maccy-kde.desktop");
        let dst = std::path::Path::new(&out_dir).join("maccy-kde.desktop");
        std::fs::copy(src, dst).unwrap();
    }
}
