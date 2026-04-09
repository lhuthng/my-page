import { route } from "$lib/server/proxy";
import { error } from "@sveltejs/kit";

/**
 * Decode the payload of a JWT without verifying the signature.
 * Safe here because the token was issued and will be re-validated by the
 * backend on every actual data request. We only use it to read the `role`
 * claim so we can guard the dashboard layout without an extra network call.
 */
function parseJwtClaims(token) {
  try {
    // JWT uses base64url (- instead of +, _ instead of /), no padding
    const base64Url = token.split(".")[1];
    const base64 = base64Url.replace(/-/g, "+").replace(/_/g, "/");
    return JSON.parse(atob(base64));
  } catch {
    return null;
  }
}

export async function load(event) {
  const refreshToken = event.cookies.get("refresh-token");

  if (!refreshToken) {
    throw error(401, "You are not logged in. Redirecting...");
  }

  let { accessToken } = await event.parent();

  if (!accessToken) {
    const res = await event.fetch(route("auth/refresh"), {
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
      throw error(401, "Session expired. Redirecting...");
    }

    const { token_type: type, token } = await res.json();
    accessToken = { type, token };
  }

  // Read the role directly from the JWT payload — no extra API round-trip.
  // The backend re-validates the token on every protected endpoint anyway.
  const claims = parseJwtClaims(accessToken.token);
  const role = claims?.role ?? null;

  const isMod = role === "admin" || role === "moderator";
  if (!isMod) {
    throw error(
      403,
      "Unauthorized: This page is only for moderators and admins. Redirecting...",
    );
  }

  return {
    accessToken,
    role,
  };
}
