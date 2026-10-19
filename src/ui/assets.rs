use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

gpui_kit::assets::icon_assets!(
    NavIcons,
    [
        House,
        BookA,
        TextQuote,
        Palette,
        Settings,
        CircleArrowDown,
        MessageSquare,
        Sparkles,
        KeyRound,
        Mic,
        Accessibility,
        Keyboard,
        Check,
        Copy,
        Info,
        Ellipsis,
        FileText,
        Trash,
        ChevronUp,
        ChevronDown,
        RotateCw,
        X
    ]
);

pub struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        match path {
            "icons/typeaside.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/typeaside.svg"
            )))),
            "onboarding/bloom.png" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/onboarding/bloom.png"
            )))),
            "onboarding/sprig.png" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/onboarding/sprig.png"
            )))),
            _ => match NavIcons.load(path)? {
                Some(icon) => Ok(Some(icon)),
                None => gpui_kit::assets::Assets.load(path),
            },
        }
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        let mut paths = gpui_kit::assets::Assets.list(path)?;
        paths.extend(NavIcons.list(path)?);
        if "icons/typeaside.svg".starts_with(path) {
            paths.push("icons/typeaside.svg".into());
        }
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}
