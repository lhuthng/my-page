import adapter from 'svelte-adapter-bun';
import { DEMO_FRAME_ORIGINS, FRAME_ORIGINS } from './src/lib/custom-rules/embed-origins.js';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	kit: {
		// adapter-auto only supports some environments, see https://svelte.dev/docs/kit/adapter-auto for a list.
		// If your environment is not supported, or you settled on a specific environment, switch out the adapter.
		// See https://svelte.dev/docs/kit/adapters for more information about adapters.
		adapter: adapter(),
		// Every post, project and game body is rendered into `{@html}`, so a
		// single escaping bug in a markdown plugin is script execution. This
		// policy is the second line of defence: no inline script and no inline
		// event handler can run, whatever the renderer emits. Hashes rather
		// than nonces keep a rendered page cacheable, and SvelteKit hashes its
		// own bootstrap script. Turnstile is the only third-party script the
		// site loads.
		//
		// `wasm-unsafe-eval` is required to compile WebAssembly at all; it is
		// not the same as `unsafe-eval`. Without it the v86 player's main
		// module is fetched, fails to compile, and falls back to a
		// `v86-fallback.wasm` this repo does not ship, so the real error
		// surfaces as a confusing 404 for the fallback file.
		//
		// `blob:` is here for AudioWorklet modules. v86's Sound Blaster DAC
		// builds its processor from an inline string and loads it from a blob
		// URL, and worklets are checked against `script-src` rather than
		// `worker-src`, so without this entry games have no sound.
		csp: {
			mode: 'hash',
			directives: {
				'script-src': ['self', 'wasm-unsafe-eval', 'blob:', 'https://challenges.cloudflare.com'],
				'worker-src': ['self', 'blob:', 'https://challenges.cloudflare.com'],
				// Frames are limited to the players and demo hosts the site
				// actually uses; the markdown plugin enforces the same list.
				// DEMO_FRAME_ORIGINS covers the backend host the html5 demo
				// pages are framed from (production sets BACKEND_ORIGIN).
				'frame-src': ['self', ...DEMO_FRAME_ORIGINS, ...FRAME_ORIGINS],
				'object-src': ['none'],
				'base-uri': ['self'],
				'form-action': ['self']
			}
		}
	}
};

export default config;
