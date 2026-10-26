use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=resources/app.rc");
    println!("cargo:rerun-if-changed=branding/dist/symbol/app.ico");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        if let Ok(out_dir) = env::var("OUT_DIR") {
            let out_obj = Path::new(&out_dir).join("resources.o");

            let status = Command::new("windres")
                .arg("-I")
                .arg(".")
                .arg("-i")
                .arg("resources/app.rc")
                .arg("-o")
                .arg(&out_obj)
                .status();

            if let Ok(s) = status {
                if s.success() {
                    println!("cargo:rustc-link-arg={}", out_obj.display());
                }
            }
        }
    }
}
