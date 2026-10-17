import { useEffect, useState } from "react";

const release = "/releases/";
const permissionsCommand = `sudo usermod -aG input "$USER" &&
 echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' | sudo tee /etc/udev/rules.d/99-typeaside.rules &&
 echo uinput | sudo tee /etc/modules-load.d/typeaside.conf &&
 sudo modprobe uinput &&
 sudo udevadm control --reload-rules &&
 sudo udevadm trigger`;
const permissionsStep = {
  vignette: "permissions",
  caption: [
    "Enable keyboard access: ",
    { command: permissionsCommand },
    ". Sign out and back in, then open ",
    { strong: "TypeAside" },
    ".",
  ],
} as const;
export const downloads = {
  macOS: {
    requirement: "macOS 13+. Apple silicon and Intel.",
    files: [
      {
        slug: "mac",
        label: "Download for macOS",
        filename: "TypeAside.dmg",
        href: `${release}TypeAside.dmg`,
        steps: [
          {
            vignette: "downloads",
            caption: [
              "Open ",
              { strong: "TypeAside.dmg" },
              " from your ",
              { strong: "Downloads" },
              " folder.",
            ],
          },
          {
            vignette: "applications",
            caption: ["Drag ", { strong: "TypeAside" }, " into ", { strong: "Applications" }, "."],
          },
          {
            vignette: "privacy",
            caption: [
              "Open ",
              { strong: "TypeAside" },
              ". If macOS blocks it, go to ",
              { strong: "System Settings › Privacy & Security" },
              " and click ",
              { strong: "Open Anyway" },
              ".",
            ],
          },
        ],
      },
    ],
  },
  Windows: {
    requirement: "Windows 10 and 11.",
    files: [
      {
        slug: "windows",
        label: "Download for Windows",
        filename: "TypeAside-setup.exe",
        href: `${release}TypeAside-setup.exe`,
        steps: [
          {
            vignette: "downloads",
            caption: [
              "Run ",
              { strong: "TypeAside-setup.exe" },
              " from your ",
              { strong: "Downloads" },
              " folder.",
            ],
          },
          {
            vignette: "smartscreen",
            caption: [
              "If SmartScreen appears, click ",
              { strong: "More info" },
              ", then ",
              { strong: "Run anyway" },
              ".",
            ],
          },
          {
            vignette: "launcher",
            caption: [
              "Open ",
              { strong: "TypeAside" },
              " from the ",
              { strong: "Start" },
              " menu.",
            ],
          },
        ],
      },
    ],
  },
  Linux: {
    requirement: "Linux with Wayland.",
    files: [
      {
        slug: "appimage",
        label: "AppImage",
        filename: "TypeAside.AppImage",
        href: `${release}TypeAside.AppImage`,
        steps: [
          {
            vignette: "downloads",
            caption: ["Open a terminal in your ", { strong: "Downloads" }, " folder."],
          },
          {
            vignette: "terminal",
            command: "chmod +x TypeAside.AppImage && ./TypeAside.AppImage",
            caption: [
              "Run ",
              { command: "chmod +x TypeAside.AppImage && ./TypeAside.AppImage" },
              ".",
            ],
          },
          permissionsStep,
        ],
      },
      {
        slug: "deb",
        label: ".deb",
        filename: "typeaside.deb",
        href: `${release}typeaside.deb`,
        steps: [
          {
            vignette: "downloads",
            caption: ["Open a terminal in your ", { strong: "Downloads" }, " folder."],
          },
          {
            vignette: "terminal",
            command: "sudo apt install ./typeaside.deb",
            caption: ["Run ", { command: "sudo apt install ./typeaside.deb" }, "."],
          },
          permissionsStep,
        ],
      },
    ],
  },
} as const;
export const downloadFiles = Object.values(downloads).flatMap(({ files }) => [...files]);
export type DownloadFile = (typeof downloadFiles)[number];
export type InstallStep = DownloadFile["steps"][number];
export const releaseFiles = new Set([
  "latest.json",
  ...Object.values(downloads).flatMap(({ files }) =>
    files.map(({ href }) => href.slice(release.length)),
  ),
]);
export function detectOS(agent: string) {
  if (/Macintosh|Mac OS X/.test(agent) && !/iPhone|iPad/.test(agent)) return "macOS";
  if (/Windows/.test(agent)) return "Windows";
  if (/Linux/.test(agent) && !/Android/.test(agent)) return "Linux";
  return null;
}
export function useOS() {
  const [os, setOS] = useState<ReturnType<typeof detectOS>>(null);
  useEffect(() => {
    setOS(detectOS(navigator.userAgent));
  }, []);
  return os;
}
