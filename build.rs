//! Embeds the application icon in the Windows executable.
//!
//! gpui loads the window and taskbar icon from resource ID 1 of the module.

fn main() {
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/icons/app.ico");

    if std::env::var_os("CARGO_CFG_TARGET_OS").is_some_and(|os| os == "windows") {
        embed_resource::compile("assets/app.rc", embed_resource::NONE)
            .manifest_optional()
            .expect("embed the application icon");
    }
}
