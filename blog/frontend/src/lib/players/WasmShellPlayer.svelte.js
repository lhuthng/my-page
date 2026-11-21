// Bridges an embedded wasmshell bundle (e.g. the Vice City hosting bundle) to
// the site's save store, over the postMessage protocol its shell.js documents.
//
// The shell owns the game and its IDBFS persistence: a guest gets local saves
// with no help from this page. The bridge only exists so a signed-in reader's
// saves follow them between devices — the shell reports what changed
// (`saves:changed`), and this side mirrors it into the account's cloud files;
// before boot it replays the account's files back in (`saves:import`).
// Messages are same-origin only, and the auth token never crosses the
// boundary: the cloud fetches happen here and only file bytes move.

import { auth } from '$lib/auth/user.svelte.js';
import { listSaves, loadSaveFile, saveSaveFile, deleteSaveFile } from '$lib/saves/game-saves.js';

/** The bundle's page URL, slash-safe whatever the caller passed. */
export function shellEntryUrl(payloadUrl, entry) {
	const base = payloadUrl ?? '';
	const normalized = base.endsWith('/') ? base : `${base}/`;
	return normalized + (entry ?? 'index.html');
}

export class WasmShellBridge {
	/** @param {string} slug the game's slug; save files are keyed by it */
	constructor({ slug, onDiagnostic = () => {} }) {
		this.slug = slug;
		this.onDiagnostic = onDiagnostic;
		this.frame = null;
		this.ready = false;
		this.imported = false;
		this.prefetched = null; // Promise<{name, bytes}[]> | null
		this.handler = (event) => this.receive(event);
	}

	attach(frame) {
		this.frame = frame;
		window.addEventListener('message', this.handler);
		// The cloud files are fetched while the bundle's assets load, so the
		// import lands the moment the shell says it is ready rather than a
		// network round trip later.
		if (auth()) {
			this.prefetched = (async () => {
				const keys = await listSaves(this.slug);
				const files = [];
				for (const key of keys) {
					const bytes = await loadSaveFile(this.slug, key);
					if (bytes) files.push({ name: key, data: bytes });
				}
				return files;
			})();
		}
	}

	detach() {
		window.removeEventListener('message', this.handler);
		this.frame = null;
	}

	/** Whether a message came from our own frame, on our own origin. */
	fromFrame(event) {
		return this.frame && event.source === this.frame.contentWindow;
	}

	async receive(event) {
		if (!this.fromFrame(event) || event.origin !== window.location.origin) return;
		const message = event.data;
		if (!message || typeof message !== 'object') return;

		if (message.type === 'saves:ready' && !this.imported) {
			this.ready = true;
			await this.importCloudSaves();
		} else if (message.type === 'saves:changed') {
			await this.mirrorChanged(message.files ?? []);
		}
	}

	/** Replays the account's save files into the shell before it boots. */
	async importCloudSaves() {
		this.imported = true;
		if (!this.prefetched) return;
		try {
			const files = await this.prefetched;
			if (!files.length || !this.frame?.contentWindow) return;
			this.frame.contentWindow.postMessage({ type: 'saves:import', files }, window.location.origin);
			this.onDiagnostic(`imported ${files.length} cloud save file(s)`);
		} catch (error) {
			this.onDiagnostic(`cloud import failed: ${error.message ?? error}`);
		}
	}

	/** Mirrors the shell's save writes into the site's save store. */
	async mirrorChanged(files) {
		if (!auth()) return; // A guest's saves live in the shell's own IndexedDB.
		for (const file of files) {
			try {
				if (file.data == null) {
					await deleteSaveFile(this.slug, file.name);
				} else {
					await saveSaveFile(this.slug, file.name, file.data);
				}
			} catch (error) {
				this.onDiagnostic(`save ${file.name}: ${error.message ?? error}`);
			}
		}
	}

	/**
	 * Clears every save of this game: the account's cloud copies here, and the
	 * shell's own IDBFS files through the bridge message its shell documents.
	 * The shell reports the removals back, which re-deletes (harmlessly) the
	 * cloud rows already gone.
	 */
	async clearNow() {
		if (auth()) {
			try {
				const keys = await listSaves(this.slug);
				for (const key of keys) {
					await deleteSaveFile(this.slug, key);
				}
			} catch (error) {
				this.onDiagnostic(`clear failed: ${error.message ?? error}`);
			}
		}
		this.post({ type: 'saves:clear' });
	}

	/** Reloads the framed bundle: the shell boots fresh from its own page. */
	restart() {
		this.ready = false;
		this.imported = false;
		this.prefetched = null;
		if (this.frame) {
			// Re-assigning the src re-navigates the frame without rebuilding it.
			this.frame.src = this.frame.src;
		}
		if (auth()) {
			this.prefetched = (async () => {
				const keys = await listSaves(this.slug);
				const files = [];
				for (const key of keys) {
					const bytes = await loadSaveFile(this.slug, key);
					if (bytes) files.push({ name: key, data: bytes });
				}
				return files;
			})();
		}
	}

	post(message) {
		this.frame?.contentWindow?.postMessage(message, window.location.origin);
	}
}
