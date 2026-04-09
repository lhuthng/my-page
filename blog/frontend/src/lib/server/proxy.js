import { env } from "$env/dynamic/private";
import { error } from "@sveltejs/kit";

export function route(path) {
  return `${env.API_URL}/${path}`;
}

/**
 * Convert a backend media path (e.g. "media/i/my-slug") into a URL the
 * browser can fetch directly.
 *
 * When PUBLIC_BACKEND_URL is set (e.g. "https://my-blog-backend.fly.dev"),
 * the browser fetches the image straight from the backend, skipping the
 * SvelteKit proxy entirely — one fewer network hop, no memory pressure on
 * the frontend machine, and proper Cache-Control headers reach the browser
 * without being rewritten.
 *
 * When PUBLIC_BACKEND_URL is not set (local dev or not yet configured), it
 * falls back to the existing /api/... proxy route so nothing breaks.
 */
export function fixClientRoute(path) {
  if (path == undefined) return undefined;
  const publicBackend = env.PUBLIC_BACKEND_URL;
  if (publicBackend) {
    return `${publicBackend.replace(/\/$/, "")}/${path}`;
  }
  return `/api/${path}`.replace("/./", "/");
}

export async function proxyFallback({ request, params, search, extraHeaders }) {
  const url = `${env.API_URL}/${params.path}${search ?? ""}`;

  const proxyRequest = new Request(url, {
    headers: { ...Object.fromEntries(request.headers), ...extraHeaders },
    method: request.method,
    body: request.body,
    duplex: "half",
    cache: request.cache,
    credentials: request.credentials,
    integrity: request.integrity,
    keepalive: request.keepalive,
    mode: request.mode,
    redirect: request.redirect,
    referrer: request.referrer,
    referrerPolicy: request.referrerPolicy,
  });

  try {
    const response = await fetch(proxyRequest);

    return response;
  } catch (e) {
    console.error("API Proxy Error:", e);
    throw error(503, "Backend service unavailable.");
  }
}
