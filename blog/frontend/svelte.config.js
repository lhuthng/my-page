import adapter from 'svelte-adapter-bun';

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
		csp: {
			mode: 'hash',
			directives: {
				'script-src': ['self', 'https://challenges.cloudflare.com'],
				'worker-src': ['self', 'blob:', 'https://challenges.cloudflare.com']
			}
		}
	}
};

export default config;
