use gpui::{AssetSource, SharedString};
use std::borrow::Cow;

gpui_kit::assets::icon_assets!(
    NavIcons,
    [
        House,
        Search,
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
            "icons/notebook-pen.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/notebook-pen.svg"
            )))),
            "icons/pin-filled.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/pin-filled.svg"
            )))),
            "icons/archive.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/archive.svg"
            )))),
            "icons/archive-restore.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/archive-restore.svg"
            )))),
            "icons/palette.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/palette.svg"
            )))),
            "icons/square.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/square.svg"
            )))),
            "icons/square-check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/square-check.svg"
            )))),
            "icons/check.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/check.svg"
            )))),
            "icons/pin.svg" => Ok(Some(Cow::Borrowed(include_bytes!(
                "../../assets/icons/pin.svg"
            )))),
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
        for icon in [
            "typeaside",
            "notebook-pen",
            "pin",
            "pin-filled",
            "archive",
            "archive-restore",
            "palette",
            "square",
            "square-check",
            "check",
        ] {
            let icon = format!("icons/{icon}.svg");
            if icon.starts_with(path) {
                paths.push(icon.into());
            }
        }
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}
