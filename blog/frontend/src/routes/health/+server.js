import { json } from '@sveltejs/kit';

// Liveness probe for the Docker healthcheck: no backend calls, no state.
export function GET() {
	return json({ status: 'ok' });
}
