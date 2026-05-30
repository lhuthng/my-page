import { error } from "@sveltejs/kit";
import { fixClientRoute, route } from "$lib/server/proxy";

export async function load({ fetch, params, setHeaders }) {
  const res = await fetch(route(`tags/${encodeURIComponent(params.slug)}?limit=100`), {
    method: "GET",
  });

  if (res.status === 404) {
    throw error(404, "Tag not found");
  }

  if (!res.ok) {
    throw error(res.status, await res.text());
  }

  setHeaders({
    "cache-control": "public, max-age=60, s-maxage=60",
  });

  const data = await res.json();

  data?.posts?.forEach((post) => {
    if (post.url) post.url = fixClientRoute(post.url);
  });

  return data;
}
