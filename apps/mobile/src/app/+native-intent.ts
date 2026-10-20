export function redirectSystemPath({ path }: { path: string; initial: boolean }) {
  try {
    const url = new URL(path, "app.typeaside:///");
    const route =
      url.protocol === "app.typeaside:" && url.hostname
        ? `/${url.hostname}${url.pathname}`
        : url.pathname;
    const note = /^\/notes\/([^/]+)\/?$/.exec(route);
    if (note?.[1]) return `/note/${note[1]}${url.search}`;
    if (route === "/dictate") return "/note/new?dictate=1";
    if (route === "/account") return `/account${url.search}`;
    if (route === "/notes" || route === "/history") return `/(app)/(tabs)${route}${url.search}`;
    return path;
  } catch {
    return path;
  }
}
