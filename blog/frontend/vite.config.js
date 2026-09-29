import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

/// The dev server's port when `PORT` says nothing. Local development pairs it
/// with the backend on 5174 (see backend/example.env), and Vite still steps to
/// the next free port if this one is taken.
const DEV_PORT = 5175;

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	// `PORT` moves the dev server too, not just the built one: it is what the
	// Bun adapter reads in production (docker-compose sets 8080), so one
	// variable decides the port whichever way this app is running.
	server: { port: Number(process.env.PORT) || DEV_PORT },
	preview: { port: Number(process.env.PORT) || DEV_PORT },
	ssr: { noExternal: ['three'] },
	build: {
		chunkSizeWarningLimit: 1200
	}
});
