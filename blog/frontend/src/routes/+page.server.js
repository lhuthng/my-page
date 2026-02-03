import { fixClientRoute, route } from "$lib/server/proxy.js";

export async function load({ fetch, setHeaders }) {
  const res = await fetch(route(`posts/featured?limit=${5}`), {
    method: "GET",
    headers: {
      "Content-Type": "application/json",
    },
  });

  if (res.ok) {
    setHeaders({
      "cache-control": "public, max-age=10, s-maxage=10",
    });
    const data = await res.json();
    data?.featured_posts?.forEach((post) => {
      if (post.url) post.url = fixClientRoute(post.url);
    });
    return data;
  } else {
    console.log(await res.text());
  }
  return {};
}
