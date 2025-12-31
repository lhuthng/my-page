import { env } from "$env/dynamic/private";
import { fixClientRoute, route } from "$lib/server/proxy";

export async function handle({ event, resolve }) {
  const { API_URL } = env;
  const accept = event.request.headers.get("accept") ?? "";
  if (accept.includes("text/html")) {
    const refreshToken = event.cookies.get("refresh-token");

    if (!refreshToken) {
      event.locals.accessToken = null;
      event.locals.user = null;
      event.locals.role = null;
      return await resolve(event);
    }

    let res = await event.fetch(route("auth/refresh"), {
      method: "POST",
      headers: {
        cookie: `refresh-token=${refreshToken ?? ""}`,
      },
    });

    if (!res.ok) {
      event.cookies.delete("refresh-token", {
        path: "/",
        httpOnly: true,
        secure: true,
      });
      event.locals.user = null;
      return resolve(event);
    }

    const { token_type: type, token } = await res.json();

    res = await event.fetch(route("users/me"), {
      method: "GET",
      headers: {
        Authorization: `${type} ${token}`,
        "Content-Type": "application/json",
      },
    });

    if (!res.ok) {
      event.locals.user = null;
      return resolve(event);
    }

    const {
      username,
      display_name: displayName,
      role,
      avatar_url: avatarUrl,
    } = await res.json();

    event.locals.accessToken = { type, token };
    event.locals.user = {
      username,
      displayName,
      role,
      avatarUrl: fixClientRoute(avatarUrl),
    };
    event.locals.role = role;
  }

  return resolve(event);
}
