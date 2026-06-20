import { json } from '@sveltejs/kit';
import { env } from '$env/dynamic/private';
import { createRateLimiter } from '$lib/server/rate-limiter.js';
import { isValidGuestIdentity } from '$lib/features/comments/guest-identities.js';
import { setGuestMapping } from '$lib/server/guest-mapping.js';

const rateLimiter = createRateLimiter({ maxRequests: 3, windowMs: 10 * 60 * 1000 });

export async function PUT({ request, params, getClientAddress }) {
  const bodyText = await request.text();
  const body = JSON.parse(bodyText);
  const headers = new Headers(request.headers);

  if (!headers.has('Authorization')) {
    if (!body.guest_identity || !isValidGuestIdentity(body.guest_identity)) {
      return json({ error: 'Invalid guest identity' }, { status: 400 });
    }
    const clientIp = getClientAddress();
    const rateResult = rateLimiter.check(`guest_comment:${clientIp}`);
    if (!rateResult.allowed) {
      return json({ error: 'Too many comments. Please wait before posting again.' }, { status: 429 });
    }
  }

  const url = `${env.API_URL}/posts/id/${params.id}/comments/new`;
  const proxyRequest = new Request(url, {
    method: 'PUT',
    headers,
    body: JSON.stringify(body),
    duplex: 'half'
  });

  let res;
  try {
    res = await fetch(proxyRequest);
  } catch (e) {
    console.error('API Proxy Error:', e);
    return new Response('Backend service unavailable.', { status: 503 });
  }

  if (!res.ok) {
    const text = await res.text();
    return new Response(text, { status: res.status });
  }

  const data = await res.json();
  if (body.guest_identity && data.comment_id) {
    setGuestMapping(data.comment_id, body.guest_identity);
  }
  return json(data);
}
