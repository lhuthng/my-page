import { redirect } from '@sveltejs/kit';

export async function load(event) {
	const { role, accessToken } = await event.parent();

	if (role !== 'admin') {
		redirect(303, '/dashboard');
	}

	return { accessToken };
}
