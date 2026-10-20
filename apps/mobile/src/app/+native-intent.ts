export function redirectSystemPath({ path }: { path: string; initial: boolean }) {
  try {
    const url = new URL(path, "app.typeaside:///");
    const route =
      url.protocol === "app.typeaside:" && url.hostname
        ? `/${url.hostname}${url.pathname}`
        : url.pathname;
    const note = /^\/notes\/([^/]+)\/?$/.exec(route);
    if (note?.[1]) return `/note/${note[1]}${url.search}`;
    if (route === "/dictate" || route === "/notes" || route === "/account")
      return `/(tabs)${route}${url.search}`;
    return path;
  } catch {
    return path;
  }
}
