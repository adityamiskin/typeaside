import { useEffect, useState } from "react";

const release = "/releases/";
export const downloads = {
  macOS: {
    requirement: "macOS 13+. Apple silicon and Intel.",
    files: [{ slug: "mac", label: "Download for macOS", href: `${release}TypeAside.dmg` }],
  },
  Windows: {
    requirement: "Windows 10 and 11.",
    files: [
      { slug: "windows", label: "Download for Windows", href: `${release}TypeAside-setup.exe` },
    ],
  },
  Linux: {
    requirement: "Linux with Wayland.",
    files: [
      { slug: "appimage", label: "AppImage", href: `${release}TypeAside.AppImage` },
      { slug: "deb", label: ".deb", href: `${release}typeaside.deb` },
    ],
  },
} as const;
export const downloadFiles = Object.values(downloads).flatMap(({ files }) => [...files]);
export const releaseFiles = new Set([
  "latest.json",
  ...downloadFiles.map(({ href }) => href.slice(release.length)),
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
