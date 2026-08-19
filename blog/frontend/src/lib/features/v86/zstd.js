/**
 * Reusable zstd compressor backed by the vendored module worker at
 * /zstd/zstd-compress-worker.js.  A pool of workers is kept alive and
 * requests are dispatched to any free worker so compression parallelises
 * across CPU cores.  The WASM runtime is only loaded once per page session.
 */

const WORKER_URL = '/zstd/zstd-compress-worker.js';
const WASM_URL = '/zstd/zstd.wasm';
const LEVEL = 19;

let pool = null;

function createWorker() {
	const w = new Worker(WORKER_URL, { type: 'module' });
	return { worker: w, busy: false };
}

function compressOne(buffer) {
	return new Promise((resolve, reject) => {
		if (pool === null) {
			pool = [];
		}
		// Find a free worker or queue after the last one.
		const slot =
			pool.find((s) => !s.busy) ??
			(() => {
				const s = createWorker();
				pool.push(s);
				return s;
			})();
		slot.busy = true;
		const w = slot.worker;
		const onError = (event) => {
			w.removeEventListener('message', onMessage);
			w.removeEventListener('error', onError);
			slot.busy = false;
			reject(new Error(event?.message ?? 'The zstd worker failed to start.'));
		};
		const onMessage = (event) => {
			const message = event.data;
			if (message.type === 'started') return;
			w.removeEventListener('message', onMessage);
			w.removeEventListener('error', onError);
			slot.busy = false;
			if (message.type === 'error') {
				reject(new Error(message.message ?? 'zstd compression failed.'));
				return;
			}
			if (message.type === 'done') {
				resolve(new Uint8Array(message.compressed));
				return;
			}
			reject(new Error('Unexpected message from the zstd worker.'));
		};
		w.addEventListener('message', onMessage);
		w.addEventListener('error', onError);
		// Not transferred: the caller keeps ownership of `buffer`.
		w.postMessage({ buffer, level: LEVEL, wasmUrl: WASM_URL });
	});
}

/**
 * Returns `(Uint8Array) => Promise<Uint8Array>` compressing one buffer at
 * zstd level 19.  Calls are dispatched to any free worker in the pool
 * so compression parallelises across cores.
 *
 * @param {{ workers?: number }} [opts]
 */
export function createZstdCompress({ workers = 1 } = {}) {
	if (pool === null) {
		pool = Array.from({ length: workers }, () => createWorker());
	} else if (workers > pool.length) {
		while (pool.length < workers) pool.push(createWorker());
	}
	return (buffer) => compressOne(buffer);
}

/** Terminates all pooled workers, releasing their WASM memory. */
export function teardownZstd() {
	if (pool) {
		for (const slot of pool) slot.worker.terminate();
		pool = null;
	}
}
