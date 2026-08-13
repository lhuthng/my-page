import { buildSandboxIso } from './sandbox-disk.js';

// Off the main thread: inflating and writing a few hundred megabytes would
// otherwise freeze the page for the whole build.
self.onmessage = async ({ data }) => {
	try {
		const { image, plan, files, bytes } = await buildSandboxIso(data.file, (progress) =>
			self.postMessage({ type: 'progress', ...progress })
		);
		self.postMessage({ type: 'done', image: image.buffer, plan, files, bytes }, [image.buffer]);
	} catch (error) {
		self.postMessage({ type: 'error', message: error?.message ?? String(error) });
	}
};
