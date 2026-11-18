//! Embedded assets for the gpui shell: Lucide icons plus Wheelie extras.

use gpui::{AssetSource, SharedString};
use gpui_icons::LucideAssetSource;
use std::borrow::Cow;

/// Asset source backing `svg()` lookups: the Lucide catalog owns
/// `icons/lucide/`, Wheelie keeps its own files under `icons/`.
pub struct WheelieAssets;

impl AssetSource for WheelieAssets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        match LucideAssetSource.load(path) {
            Ok(Some(asset)) => Ok(Some(asset)),
            Ok(None) => {
                if let Some(asset) = own_asset(path) {
                    return Ok(Some(asset));
                }
                // Kit components (Input menus, icons) resolve their own files.
                gpui_kit::assets::AllAssets.load(path)
            }
            Err(error) => Err(error),
        }
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        match path {
            "icons/lucide" | "icons/lucide/" => LucideAssetSource.list(path),
            _ => Ok(match path {
                "icons" => vec![SharedString::from("plus.svg")],
                _ => Vec::new(),
            }),
        }
    }
}

/// Wheelie-owned embedded files, outside the shared Lucide catalog.
fn own_asset(path: &str) -> Option<Cow<'static, [u8]>> {
    match path {
        "icons/plus.svg" => Some(Cow::Borrowed(include_bytes!("../../assets/icons/plus.svg"))),
        "logo/logo-2x.png" => Some(Cow::Borrowed(include_bytes!(
            "../../assets/logo/logo-2x.png"
        ))),
        _ => None,
    }
}

/// Registers the bundled Rubik family with the gpui text system.
pub fn install_fonts(gpui_app: &gpui::App) -> anyhow::Result<()> {
    use std::borrow::Cow as StdCow;

    gpui_app.text_system().add_fonts(vec![
        StdCow::Borrowed(include_bytes!("../../assets/fonts/Rubik-Regular.ttf").as_slice()),
        StdCow::Borrowed(include_bytes!("../../assets/fonts/Rubik-SemiBold.ttf").as_slice()),
    ])?;
    Ok(())
}
