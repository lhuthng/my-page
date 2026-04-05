import { fixClientRoute, route } from "$lib/server/proxy";

export async function load(event) {
  const { accessToken } = await event.parent();
  const limit = 20;

  try {
    const res = await event.fetch(
      route(`dashboard/users?limit=${limit}&offset=0`),
      {
        headers: {
          Authorization: `${accessToken.type} ${accessToken.token}`,
        },
      },
    );

    if (!res.ok)
      return { users: [], total: 0, role_counts: { admin: 0, moderator: 0, user: 0 } };

    const data = await res.json();

    data.users?.forEach((u) => {
      u.avatar_url = fixClientRoute(u.avatar_url);
    });

    return {
      users: data.users ?? [],
      total: data.total ?? 0,
      role_counts: data.role_counts ?? { admin: 0, moderator: 0, user: 0 },
    };
  } catch {
    return { users: [], total: 0, role_counts: { admin: 0, moderator: 0, user: 0 } };
  }
}
