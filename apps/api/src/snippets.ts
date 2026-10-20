type Snippet = { trigger: string; text: string };

export function expandSnippets(text: string, snippets: Snippet[]) {
  const patterns = snippets
    .map((snippet) => ({
      ...snippet,
      trigger: snippet.trigger.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, ""),
    }))
    .filter((snippet) => snippet.trigger)
    .sort(
      (a, b) =>
        new TextEncoder().encode(b.trigger).length - new TextEncoder().encode(a.trigger).length,
    )
    .map((snippet) => ({
      ...snippet,
      pattern: new RegExp(snippet.trigger.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "iyu"),
    }));
  const whole = text
    .replace(/^\p{White_Space}+|\p{White_Space}+$/gu, "")
    .replace(/[.!?,]+$/u, "")
    .replace(/\p{White_Space}+$/u, "");
  for (const snippet of patterns) {
    snippet.pattern.lastIndex = 0;
    if (snippet.pattern.exec(whole)?.[0].length === whole.length) return snippet.text;
  }
  const word = (value: string | undefined) =>
    value !== undefined && /[\p{Alphabetic}\p{N}_]/u.test(value);
  let output = "";
  let offset = 0;
  let previous: string | undefined;
  while (offset < text.length) {
    const match = patterns.find((snippet) => {
      snippet.pattern.lastIndex = offset;
      const found = snippet.pattern.exec(text);
      return (
        found !== null &&
        !word(previous) &&
        !word(String.fromCodePoint(text.codePointAt(offset + found[0].length) ?? 32))
      );
    });
    if (match) {
      const end = match.pattern.lastIndex;
      previous = Array.from(text.slice(offset, end)).at(-1);
      output += match.text;
      offset = end;
    } else {
      const character = String.fromCodePoint(text.codePointAt(offset)!);
      output += character;
      previous = character;
      offset += character.length;
    }
  }
  return output;
}
