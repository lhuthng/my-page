import { fixClientRoute, route } from "$lib/server/proxy.js";

export async function load(event) {
  const { accessToken } = await event.parent();
  const { type, token } = accessToken;

  try {
    const res = await event.fetch(route("series/all"), {
      method: "GET",
      headers: { Authorization: `${type} ${token}` },
    });

    if (!res.ok) return { series: [] };

    const { series } = await res.json();
    series.forEach((s) => {
      s.url = fixClientRoute(s.url);
    });

    return { series };
  } catch {
    return { series: [] };
  }
}
