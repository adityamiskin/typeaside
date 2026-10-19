use std::{env, fs, path::PathBuf};

pub(super) fn resolve(id: &str) -> String {
    let home = env::var_os("HOME").map(PathBuf::from);
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|home| home.join(".local/share")));
    let data_dirs =
        env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    let dirs = data_home
        .into_iter()
        .chain(env::split_paths(&data_dirs))
        .chain(home.map(|home| home.join(".local/share/flatpak/exports/share")))
        .chain([PathBuf::from("/var/lib/flatpak/exports/share")]);
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir.join("applications")) else {
            continue;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|entry| {
            !entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(&format!("{id}.desktop"))
        });
        for entry in entries {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "desktop") {
                continue;
            }
            let Ok(contents) = fs::read_to_string(&path) else {
                continue;
            };
            if let Some(name) = entry_name(&entry.file_name().to_string_lossy(), &contents, id) {
                return name.to_owned();
            }
        }
    }
    super::super::fallback_app_name(id)
}

fn entry_name<'a>(filename: &str, contents: &'a str, id: &str) -> Option<&'a str> {
    let mut desktop_entry = false;
    let mut name = None;
    let mut matches = filename
        .strip_suffix(".desktop")
        .is_some_and(|stem| stem.eq_ignore_ascii_case(id));
    for line in contents.lines().map(str::trim) {
        if line.starts_with('[') {
            desktop_entry = line == "[Desktop Entry]";
        } else if desktop_entry {
            if let Some(value) = line.strip_prefix("Name=").filter(|value| !value.is_empty()) {
                name = Some(value);
            }
            if let Some(value) = line.strip_prefix("StartupWMClass=") {
                matches |= value.eq_ignore_ascii_case(id);
            }
        }
    }
    matches.then_some(name).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_desktop_identity_and_name() {
        let entry = "[Desktop Entry]\nName=T3 Code\nName[fr]=Autre\nStartupWMClass=T3Code\n[Desktop Action New]\nName=New Window\n";
        for id in ["com.t3tools.T3Code", "COM.T3TOOLS.T3CODE", "t3code"] {
            assert_eq!(
                entry_name("com.t3tools.T3Code.desktop", entry, id),
                Some("T3 Code")
            );
        }
        assert_eq!(entry_name("other.desktop", entry, "unknown"), None);
        assert_eq!(
            entry_name("app.desktop", "[Desktop Entry]\nName[fr]=Autre", "app"),
            None
        );
    }
}
