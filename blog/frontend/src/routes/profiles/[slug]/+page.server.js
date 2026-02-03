import { fixClientRoute, route } from "$lib/server/proxy.js";
import { error } from "@sveltejs/kit";

export async function load({ fetch, params, setHeaders }) {
  const username = params.slug;

  const res = await fetch(route(`users/${username}`), {
    method: "GET",
  });

  if (res.ok) {
    setHeaders({
      "cache-control": "public, max-age=60, s-maxage=60",
    });
    const response = await res.json();
    response.avatar_url = fixClientRoute(response.avatar_url);
    return { response };
  } else {
    error(404, { message: "Not found" });
  }
}
