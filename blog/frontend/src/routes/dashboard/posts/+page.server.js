import { fixClientRoute, route } from "$lib/server/proxy";

export async function load(event) {
  const { accessToken } = await event.parent();
  const limit = 9;

  try {
    const res = await event.fetch(
      route(`dashboard/posts?limit=${limit}&offset=0`),
      {
        headers: {
          Authorization: `${accessToken.type} ${accessToken.token}`,
        },
      },
    );

    if (!res.ok) return { posts: [], total: 0 };

    const data = await res.json();

    data.posts?.forEach((p) => {
      p.url = fixClientRoute(p.url);
    });

    return {
      posts: data.posts ?? [],
      total: data.total ?? 0,
    };
  } catch {
    return { posts: [], total: 0 };
  }
}
