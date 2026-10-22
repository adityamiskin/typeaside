// A lone "send" submits without pasting text.
export function trailingSend(raw: string) {
  const match = /(?:^|[^\p{L}\p{N}\p{M}\p{Pc}])(send)\p{P}*\s*$/iu.exec(raw);
  if (!match) return null;
  const at = match.index + match[0].toLowerCase().indexOf("send");
  return raw.slice(0, at).trimEnd();
}
