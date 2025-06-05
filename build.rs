use std::{env, path::PathBuf};

fn main() {
    if env::var("CARGO_CFG_WINDOWS").is_ok() {
        return;
    }
    println!("cargo:rerun-if-env-changed=CARGO_PKG_NAME");

    let target = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target = target
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("Failed to find target dir");
    let soname = format!("{}.so", env::var("CARGO_PKG_NAME").unwrap());

    println!("cargo:rustc-link-arg-cdylib=-o");
    println!(
        "cargo:rustc-link-arg-cdylib={}",
        target.join(&soname).to_str().unwrap()
    );
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "macos" {
        // Set the macOS "install name"
        println!("cargo:rustc-link-arg-cdylib=-Wl,-install_name,{}", &soname);
    } else {
        // Set the SONAME.
        println!("cargo:rustc-link-arg-cdylib=-Wl,-soname,{}", &soname);
    }
}
