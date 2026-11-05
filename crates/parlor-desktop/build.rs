// SPDX-License-Identifier: GPL-3.0-or-later
//! Embeds the application icon and the GStreamer plugin in the executable.
//!
//! Windows shows the executable's icon in Explorer, the taskbar and a window's title bar, so
//! the icon belongs in the resources of the program rather than in a folder beside it. The
//! GStreamer plugin is a library the program loads at run time; carrying it inside the
//! executable is what makes a release one file.

use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
    embed_plugin();
    #[cfg(windows)]
    {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("../../assets/icon.ico");
        resource.set("FileDescription", "Parlor");
        resource.set("ProductName", "Parlor");
        resource.set("LegalCopyright", "GPL-3.0-or-later");
        if let Err(error) = resource.compile() {
            // A machine without the resource compiler should still produce the application;
            // the icon is a detail, not a requirement.
            println!("cargo:warning=the icon could not be embedded: {error}");
        }
    }
}

/// Copies the compiled plugin into the build directory, where `include_bytes!` finds it.
///
/// The plugin is built first by the scripts that make a release; a plain `cargo build` of
/// the program has no plugin to copy, and writes an empty file instead. The application
/// treats an empty copy as "no embedded plugin" and falls back to a library beside it.
fn embed_plugin() {
    let name = "parlor_gst_backend.dll";
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("a build has an output directory"));
    let destination = out.join(name);
    let profile = out.ancestors().nth(3).map(Path::to_path_buf);
    let profile_plugin = profile.as_ref().map(|directory| directory.join(name));
    // Watched even when the file is not there yet: a build that first compiles the program
    // and then the plugin has to see the plugin appear and embed it.
    if let Some(path) = &profile_plugin {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let source = env::var_os("PARLOR_GST_PLUGIN").map(PathBuf::from).or(profile_plugin);
    match source {
        Some(path) if path.is_file() => {
            fs::copy(&path, &destination).expect("the plugin can be copied");
            println!("cargo:rerun-if-changed={}", path.display());
        }
        _ => {
            fs::write(&destination, []).expect("the placeholder can be written");
        }
    }
    println!("cargo:rerun-if-env-changed=PARLOR_GST_PLUGIN");
}
