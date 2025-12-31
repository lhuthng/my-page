import { fixClientRoute, route } from "$lib/server/proxy.js";

export async function load({ fetch }) {
  const res = await fetch(route(`posts/featured?limit=${5}`), {
    method: "GET",
    headers: {
      "Content-Type": "application/json",
    },
  });

  if (res.ok) {
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
