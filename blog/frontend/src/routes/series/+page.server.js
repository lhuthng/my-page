import { fixClientRoute, route } from "$lib/server/proxy.js";
import { error } from "@sveltejs/kit";

export async function load({ fetch, params }) {
  const res = await fetch(route("series/public/all"));

  if (res.ok) {
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
