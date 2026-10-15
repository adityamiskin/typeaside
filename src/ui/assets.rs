use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

gpui_kit::assets::icon_assets!(NavIcons, [House, BookA, TextQuote, Palette, Settings]);

pub struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        match NavIcons.load(path)? {
            Some(icon) => Ok(Some(icon)),
            None => gpui_kit::assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(NavIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}
