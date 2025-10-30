import { env } from "$env/dynamic/private";

export async function fallback({ request, params }) {
  const { API_URL } = env;

  const url = `${API_URL}/${params.path}`;

  console.log(url);

  const proxyRequest = new Request(url, {
    headers: request.headers,
    method: request.method,
    body: request.body,
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
