// Full-color logos from SVG Logos by Gil Barbara, CC0: https://github.com/gilbarbara/logos
export const apps = {
  gmail: "Gmail",
  slack: "Slack",
  notion: "Notion",
  cursor: "Cursor",
  chatgpt: "ChatGPT",
  claude: "Claude",
  linear: "Linear",
  github: "GitHub",
  discord: "Discord",
  whatsapp: "WhatsApp",
  figma: "Figma",
  obsidian: "Obsidian",
  drive: "Google Drive",
  teams: "Microsoft Teams",
  zoom: "Zoom",
  telegram: "Telegram",
  spotify: "Spotify",
} as const;
export type AppName = keyof typeof apps;
