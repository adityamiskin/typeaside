use crate::config::Snippet;

pub(crate) fn expand(text: &str, snippets: &[Snippet]) -> String {
    expand_with(text, snippets, || {
        crate::text_injector::read_clipboard().ok().flatten()
    })
}

fn expand_with(
    text: &str,
    snippets: &[Snippet],
    clipboard: impl FnOnce() -> Option<String>,
) -> String {
    let clipboard = std::cell::LazyCell::new(|| clipboard().unwrap_or_default());
    let render = |text: &str| {
        if text.contains("{clipboard}") {
            text.replace("{clipboard}", &clipboard)
        } else {
            text.to_owned()
        }
    };
    let whole = text
        .trim()
        .trim_end_matches(['.', '!', '?', ','])
        .trim_end();
    let mut patterns: Vec<_> = snippets
        .iter()
        .filter_map(|snippet| {
            let trigger = snippet.trigger.trim();
            if trigger.is_empty() {
                return None;
            }
            let regex = regex::RegexBuilder::new(&regex::escape(trigger))
                .case_insensitive(true)
                .build()
                .ok()?;
            Some((snippet, regex))
        })
        .collect();
    patterns.sort_by_key(|(snippet, _)| std::cmp::Reverse(snippet.trigger.trim().len()));
    for (snippet, regex) in &patterns {
        if regex
            .find(whole)
            .is_some_and(|found| found.start() == 0 && found.end() == whole.len())
        {
            return render(&snippet.text);
        }
    }
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut output = String::new();
    let mut offset = 0;
    while offset < text.len() {
        let found = patterns.iter().find_map(|(snippet, regex)| {
            let found = regex.find_at(text, offset)?;
            if found.start() != offset
                || text[..offset].chars().next_back().is_some_and(word)
                || text[found.end()..].chars().next().is_some_and(word)
            {
                return None;
            }
            Some((snippet, found.end()))
        });
        if let Some((snippet, end)) = found {
            output.push_str(&render(&snippet.text));
            offset = end;
        } else {
            let c = text[offset..]
                .chars()
                .next()
                .expect("valid character boundary");
            output.push(c);
            offset += c.len_utf8();
        }
    }
    output
}
