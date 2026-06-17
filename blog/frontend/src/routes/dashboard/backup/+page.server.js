import { redirect } from '@sveltejs/kit';

export async function load(event) {
	const { role } = await event.parent();

	if (role !== 'admin') {
		redirect(303, '/dashboard');
	}
}
