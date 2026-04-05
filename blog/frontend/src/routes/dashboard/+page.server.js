import { fixClientRoute, route } from "$lib/server/proxy";

export async function load(event) {
  const { accessToken } = await event.parent();

  try {
    const res = await event.fetch(route("dashboard/overview"), {
      headers: {
        Authorization: `${accessToken.type} ${accessToken.token}`,
      },
    });

    if (!res.ok) return { overview: null };

    const data = await res.json();

    for (const key of [
      "top_posts_by_views",
      "top_posts_by_likes",
      "top_posts_by_comments",
      "recent_posts",
    ]) {
      data[key]?.forEach((p) => {
        p.url = fixClientRoute(p.url);
      });
    }

    data.recent_users?.forEach((u) => {
      u.avatar_url = fixClientRoute(u.avatar_url);
    });

    return { overview: data };
  } catch {
    return { overview: null };
  }
}
