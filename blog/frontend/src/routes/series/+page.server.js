import { fixClientRoute, route } from "$lib/server/proxy.js";
import { error } from "@sveltejs/kit";

export async function load({ fetch, params, setHeaders }) {
  const res = await fetch(route("series/public/all"));

  if (res.ok) {
    setHeaders({
      "cache-control": "public, max-age=60, s-maxage=60",
    });
    const series = await res.json();
    series.forEach((series) => {
      series.url = fixClientRoute(series.url);
      series.posts.forEach((post) => {
        post.url = fixClientRoute(post.url);
      });
    });

    return { series };
  } else {
    console.log("not ok", await res.text());
  }
}
