fn main() {
    println!("cargo:rerun-if-changed=ui/app-window.slint");
    println!("cargo:rerun-if-changed=.assets/UseDNS.png");

    use std::{collections::HashMap, path::PathBuf};

    let lib_path = lucide_slint::lib();
    eprintln!("LUCIDE_SLINT_LIB_PATH: {}", lib_path);
    let library = HashMap::from([("lucide".to_string(), PathBuf::from(lib_path))]);
    let config = slint_build::CompilerConfiguration::new().with_library_paths(library);
    slint_build::compile_with_config("ui/app-window.slint", config)
        .expect("failed to compile Slint UI");

    #[cfg(windows)]
    embed_windows_icon();
}

#[cfg(windows)]
fn embed_windows_icon() {
    use ico::{IconDir, IconDirEntry, IconImage, ResourceType};
    use image::imageops::FilterType;
    use std::{env, fs::File, path::PathBuf};

    let source = image::open(".assets/UseDNS.png")
        .expect("failed to open application icon")
        .into_rgba8();
    let output =
        PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is not set")).join("UseDNS.ico");
    let mut icon = IconDir::new(ResourceType::Icon);

    for size in [16, 24, 32, 48, 64, 128, 256] {
        let resized = image::imageops::resize(&source, size, size, FilterType::Lanczos3);
        let image = IconImage::from_rgba_data(size, size, resized.into_raw());
        icon.add_entry(IconDirEntry::encode(&image).expect("failed to encode icon frame"));
    }

    let file = File::create(&output).expect("failed to create Windows icon");
    icon.write(file).expect("failed to write Windows icon");

    winresource::WindowsResource::new()
        .set_icon(output.to_str().expect("icon path is not valid UTF-8"))
        .compile()
        .expect("failed to embed Windows resources");
}
