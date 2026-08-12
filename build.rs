// build.rs

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let font_dir = manifest_dir.join("assets/fonts");
    let app = manifest_dir.join("ui/app.slint");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("biztrace-ui.slint");
    let font_files = [
        "Vazirmatn-Regular.ttf",
        "Vazirmatn-Medium.ttf",
        "Vazirmatn-SemiBold.ttf",
        "Vazirmatn-Bold.ttf",
    ];

    let mut source = String::from("// biztrace-ui.slint\n\n");
    for file in font_files {
        let path = font_dir.join(file);
        println!("cargo:rerun-if-changed={}", path.display());
        if path.is_file() {
            source.push_str(&format!("import \"{}\";\n", slint_path(&path)));
        }
    }
    source.push_str(&format!(
        "export {{ AppWindow }} from \"{}\";\n",
        slint_path(&app)
    ));

    fs::write(&output, source).expect("Could not generate Slint font entrypoint");
    slint_build::compile(&output).expect("Slint UI compilation failed");
}

fn slint_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
