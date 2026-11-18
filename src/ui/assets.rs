//! Embedded assets for the gpui shell: icons first, fonts alongside.

use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

/// Asset source backing `svg()` icon lookups.
pub struct WheelieAssets;

impl AssetSource for WheelieAssets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        Ok(match path {
            "icons/plus.svg" => Some(Cow::Borrowed(include_bytes!("../../assets/icons/plus.svg"))),
            _ => None,
        })
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(match path {
            "icons" => vec![SharedString::from("plus.svg")],
            _ => Vec::new(),
        })
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
