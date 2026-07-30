//! Wires up a locally-vendored libmpv (see `scripts/setup-mpv-windows.ps1`) so
//! the `mpv` feature can link on Windows without a system-wide libmpv install.
//!
//! No-op if the `mpv` feature is off, the target isn't Windows, or the vendor
//! directory hasn't been populated by the setup script.

use std::env;
use std::path::PathBuf;

fn main() {
    if env::var_os("CARGO_FEATURE_MPV").is_none() {
        return;
    }
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let vendor_dir = manifest_dir.join("..").join("vendor").join("mpv-windows");
    let lib_dir = vendor_dir.join("64");
    let dll = lib_dir.join("libmpv-2.dll");

    println!("cargo:rerun-if-changed={}", lib_dir.display());

    if !lib_dir.join("mpv.lib").exists() {
        return;
    }

    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Copy the runtime dll next to the final binaries (shared workspace
    // target/<profile> dir), OUT_DIR is target/<profile>/build/<pkg>-<hash>/out.
    if dll.exists()
        && let Ok(out_dir) = env::var("OUT_DIR")
    {
        let mut target_profile_dir = PathBuf::from(out_dir);
        for _ in 0..3 {
            target_profile_dir.pop();
        }
        let _ = std::fs::copy(&dll, target_profile_dir.join("libmpv-2.dll"));
    }
}
