export type ReplacementRule = { trigger: string; replacement: string };

const word = /^[\p{Alphabetic}\p{M}\p{Nd}\p{Pc}\p{Join_Control}]$/u;
const spaceless =
  /^[\p{sc=Han}\p{sc=Hiragana}\p{sc=Katakana}\p{sc=Thai}\p{sc=Lao}\p{sc=Khmer}\p{sc=Myanmar}]$/u;

// Scripts written without spaces have no word edges, so only joins between other word characters count.
const joined = (...pair: (string | undefined)[]) =>
  pair.every((c) => c !== undefined && word.test(c) && !spaceless.test(c));

const charAt = (text: string, index: number) => {
  const code = text.codePointAt(index);
  return code === undefined ? undefined : String.fromCodePoint(code);
};

export function applyReplacements(text: string, rules: ReplacementRule[]) {
  const patterns = rules.flatMap(({ trigger, replacement }) => {
    const literal = trigger.trim();
    if (!literal) return [];
    return [
      { pattern: new RegExp(literal.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "iuy"), replacement },
    ];
  });
  let output = "";
  let previous: string | undefined;
  let offset = 0;
  while (offset < text.length) {
    let best: { matched: string; replacement: string } | undefined;
    for (const { pattern, replacement } of patterns) {
      pattern.lastIndex = offset;
      const matched = pattern.exec(text)?.[0];
      if (
        matched &&
        !joined(previous, charAt(matched, 0)) &&
        !joined(Array.from(matched).at(-1), charAt(text, offset + matched.length)) &&
        matched.length > (best?.matched.length ?? 0)
      )
        best = { matched, replacement };
    }
    const next = charAt(text, offset)!;
    const { matched, replacement } = best ?? { matched: next, replacement: next };
    output += replacement;
    previous = Array.from(matched).at(-1);
    offset += matched.length;
  }
  return output;
}
