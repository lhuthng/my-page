import { fixClientRoute, route } from "$lib/server/proxy";

export async function load(event) {
  const firstOffset = 10;
  const res = await fetch(route(`posts/latest?limit=${firstOffset}`), {
    method: "GET",
  });

  if (res.ok) {
    const data = await res.json();

    data?.featured_posts?.forEach((post) => {
      if (post.url) post.url = fixClientRoute(post.url);
    });
    return { status: "success", firstOffset, ...data };
  } else {
    console.log(await res.text());
    return { status: "failed" };
  }
}
