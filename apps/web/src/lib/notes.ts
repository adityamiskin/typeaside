export function hashtagParts(text: string) {
  return text.split(/((?<!\S)#[\p{L}\p{N}_-]+)/u);
}

export function labels(text: string) {
  return [
    ...new Set(
      [...text.matchAll(/(?:^|\s)#([\p{L}\p{N}_-]+)/gu)].map((match) => match[1]!.toLowerCase()),
    ),
  ];
}

export function checklistItem(line: string) {
  const match = /^- \[([ x])\] (.*)$/i.exec(line);
  return match ? { checked: match[1]!.toLowerCase() === "x", text: match[2]! } : null;
}

export function toggleChecklist(text: string, index: number) {
  return text
    .split("\n")
    .map((line, i) => {
      const item = checklistItem(line);
      return i === index && item ? `- [${item.checked ? " " : "x"}] ${item.text}` : line;
    })
    .join("\n");
}
