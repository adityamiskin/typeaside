import { useEffect, useState } from "react";

const release = "/releases/";
export const downloads = {
  macOS: {
    requirement: "macOS 13+ on Apple silicon.",
    files: [{ slug: "mac", label: "Download for macOS", href: `${release}TypeAside.dmg` }],
  },
  Windows: {
    requirement: "Windows 10 and 11.",
    files: [
      { slug: "windows", label: "Download for Windows", href: `${release}TypeAside-setup.exe` },
    ],
  },
  Linux: {
    requirement: "Ubuntu 22.04+ and Fedora, on Wayland.",
    files: [
      { slug: "deb", label: "Ubuntu (.deb)", href: `${release}typeaside.deb` },
      { slug: "rpm", label: "Fedora (.rpm)", href: `${release}typeaside.rpm` },
      { slug: "appimage", label: "AppImage", href: `${release}TypeAside.AppImage` },
    ],
  },
} as const;
export const comingSoon = {
  Android: "A floating mic for any app, plus your notes.",
  iOS: "The TypeAside keyboard, plus your notes.",
} as const;
export const downloadFiles = Object.values(downloads).flatMap(({ files }) => [...files]);
export const releaseFiles = new Set([
  "latest.json",
  ...downloadFiles.map(({ href }) => href.slice(release.length)),
]);
export function detectOS(agent: string) {
  if (/Android/.test(agent)) return "Android";
  if (/iPhone|iPad/.test(agent)) return "iOS";
  if (/Macintosh|Mac OS X/.test(agent)) return "macOS";
  if (/Windows/.test(agent)) return "Windows";
  if (/Linux/.test(agent)) return "Linux";
  return null;
}
export function useOS() {
  const [os, setOS] = useState<ReturnType<typeof detectOS>>(null);
  useEffect(() => {
    setOS(detectOS(navigator.userAgent));
  }, []);
  return os;
}
