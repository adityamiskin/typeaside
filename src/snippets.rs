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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrase_replacement_table() {
        let snippets = [
            Snippet {
                trigger: "calendar".into(),
                text: "short".into(),
            },
            Snippet {
                trigger: "my calendar link".into(),
                text: "https://example.com/book".into(),
            },
            Snippet {
                trigger: "c++".into(),
                text: "Rust".into(),
            },
            Snippet {
                trigger: "short".into(),
                text: "recursive".into(),
            },
        ];
        for (input, expected) in [
            ("calendarish", "calendarish"),
            ("MY CALENDAR LINK.!?,", "https://example.com/book"),
            (
                "Use my calendar link, please.",
                "Use https://example.com/book, please.",
            ),
            ("My calendar link", "https://example.com/book"),
            ("calendar", "short"),
            ("Use C++, please.", "Use Rust, please."),
            ("xc++", "xc++"),
            ("calendar calendar", "short short"),
        ] {
            assert_eq!(
                expand_with(input, &snippets, || panic!("unexpected clipboard read")),
                expected,
                "{input}"
            );
        }
    }

    #[test]
    fn clipboard_expansion() {
        let snippets = [
            Snippet {
                trigger: "paste".into(),
                text: "[{clipboard}] {clipboard}".into(),
            },
            Snippet {
                trigger: "plain".into(),
                text: "unchanged".into(),
            },
        ];
        for (input, clipboard, expected, reads) in [
            ("PASTE.!", Some("copied"), "[copied] copied", 1),
            (
                "paste, paste!",
                Some("copied"),
                "[copied] copied, [copied] copied!",
                1,
            ),
            ("paste", None, "[] ", 1),
            ("paste", Some("{clipboard}"), "[{clipboard}] {clipboard}", 1),
            ("plain", Some("copied"), "unchanged", 0),
            (
                "pasted {clipboard}",
                Some("copied"),
                "pasted {clipboard}",
                0,
            ),
        ] {
            let calls = std::cell::Cell::new(0);
            assert_eq!(
                expand_with(input, &snippets, || {
                    calls.set(calls.get() + 1);
                    clipboard.map(str::to_owned)
                }),
                expected,
                "{input}"
            );
            assert_eq!(calls.get(), reads, "{input}");
        }
    }
}
