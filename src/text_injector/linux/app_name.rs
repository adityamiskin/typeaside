use std::{env, fs, path::PathBuf};

pub(super) fn resolve(id: &str) -> String {
    let home = env::var_os("HOME").map(PathBuf::from);
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home.as_ref().map(|home| home.join(".local/share")));
    let data_dirs =
        env::var_os("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    data_home
        .into_iter()
        .chain(env::split_paths(&data_dirs))
        .flat_map(|dir| {
            [id.to_owned(), id.to_lowercase()]
                .map(|file| dir.join("applications").join(format!("{file}.desktop")))
        })
        .find_map(|path| entry_name(&fs::read_to_string(path).ok()?).map(str::to_owned))
        .unwrap_or_else(|| super::super::fallback_app_name(id))
}

fn entry_name(contents: &str) -> Option<&str> {
    let mut desktop_entry = false;
    for line in contents.lines().map(str::trim) {
        if line.starts_with('[') {
            desktop_entry = line == "[Desktop Entry]";
        } else if desktop_entry
            && let Some(name) = line.strip_prefix("Name=").filter(|name| !name.is_empty())
        {
            return Some(name);
        }
    }
    None
}
