import { API_URL } from "$env/static/private";
import { route } from "./proxy";

export function refreshAccessToken(event, refreshToken) {
  return event
    .fetch(route("auth/refresh"), {
      method: "POST",
    })
    .then(async (res) => {
      if (!res.ok) {
        throw () => {
          event.cookies.delete("refresh-token", {
            path: "/",
            httpOnly: true,
            secure: true,
          });
          event.locals.user = null;
        };
      }
      const data = await res.json();
      return {
        type: data.type,
        token: data.access_token,
      };
    });
}

export function getCurrentUser(event, type, accessToken) {
  return event
    .fetch(route("user/me"), {
      method: "GET",
      headers: {
        Authorization: `${type} ${accessToken}`,
        "Content-Type": "application/json",
      },
    })
    .then(async (res) => {
      if (!res.ok) {
        throw () => {
          event.locals.user = null;
        };
      }
      return res.json();
    });
}
