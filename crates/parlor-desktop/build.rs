// SPDX-License-Identifier: GPL-3.0-or-later
//! Embeds the application icon and its version details in the executable.
//!
//! Windows shows the executable's icon in Explorer, the taskbar and a window's title bar, so
//! the icon belongs in the resources of the program rather than in a folder beside it.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icon.ico");
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
