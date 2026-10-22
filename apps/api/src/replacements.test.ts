import { expect, test } from "vite-plus/test";
import cases from "../../../tests/fixtures/replacements.json";
import { applyReplacements } from "./replacements";

for (const item of cases) {
  test(item.name, () => {
    const rules = item.rules.map(([trigger = "", replacement = ""]) => ({ trigger, replacement }));
    expect(applyReplacements(item.input, rules)).toBe(item.expected);
  });
}
