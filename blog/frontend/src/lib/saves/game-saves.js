import { decompress } from 'fzstd';
import { auth } from '$lib/auth/user.svelte.js';

/**
 * Unified save files for the payload launchers ('shimkit', 'wasmshell').
 *
 * One save is one guest-written file, keyed by its name in the game's own save
 * set (a shim-kit game's `Profile.dat`, Vice City's `GTAVCsf1.b`). The browser
 * always keeps its own copy in IndexedDB; a signed-in reader additionally
 * mirrors every write to the account's cloud storage, and a guest never
 * touches the server — the same split `v86-saves.js` draws for the floppies,
 * generalised to many files per game.
 *
 * The cloud endpoints are `/api/games/s/<slug>/saves[/<key>]`, proxied to the
 * backend the same way every other API call here is.
 */

const IDB_NAME = 'game-saves';
const IDB_STORE = 'saves';

function idbKey(slug, key) {
	return `${slug}/${key}`;
}

function openDb() {
	return new Promise((resolve, reject) => {
		const request = indexedDB.open(IDB_NAME, 1);
		request.onupgradeneeded = () => {
			request.result.createObjectStore(IDB_STORE);
		};
		request.onsuccess = () => resolve(request.result);
		request.onerror = () => reject(request.error);
	});
}

function idbGet(slug, key) {
	return new Promise((resolve) => {
		openDb()
			.then((db) => {
				const tx = db.transaction(IDB_STORE, 'readonly');
				const request = tx.objectStore(IDB_STORE).get(idbKey(slug, key));
				request.onsuccess = () => {
					db.close();
					const value = request.result;
					if (value instanceof Uint8Array) resolve(value);
					else if (value instanceof ArrayBuffer) resolve(new Uint8Array(value));
					else resolve(null);
				};
				request.onerror = () => {
					db.close();
					resolve(null);
				};
			})
			.catch(() => resolve(null));
	});
}

function idbPut(slug, key, bytes) {
	return new Promise((resolve) => {
		openDb()
			.then((db) => {
				const tx = db.transaction(IDB_STORE, 'readwrite');
				tx.objectStore(IDB_STORE).put(bytes, idbKey(slug, key));
				tx.oncomplete = () => {
					db.close();
					resolve(true);
				};
				tx.onerror = () => {
					db.close();
					resolve(false);
				};
			})
			.catch(() => resolve(false));
	});
}

function idbDelete(slug, key) {
	return new Promise((resolve) => {
		openDb()
			.then((db) => {
				const tx = db.transaction(IDB_STORE, 'readwrite');
				tx.objectStore(IDB_STORE).delete(idbKey(slug, key));
				tx.oncomplete = () => {
					db.close();
					resolve();
				};
				tx.onerror = () => {
					db.close();
					resolve();
				};
			})
			.catch(() => resolve());
	});
}

/** Every locally kept key for one game, read through a bounded cursor range. */
async function idbKeys(slug) {
	return new Promise((resolve) => {
		openDb()
			.then((db) => {
				const keys = [];
				const range = IDBKeyRange.bound(`${slug}/`, `${slug}/\uffff`, false, false);
				const tx = db.transaction(IDB_STORE, 'readonly');
				const request = tx.objectStore(IDB_STORE).openCursor(range);
				request.onsuccess = () => {
					const cursor = request.result;
					if (cursor) {
						// The stored key is `<slug>/<key>`; the save's own name is what
						// follows the slash.
						keys.push(cursor.key.slice(slug.length + 1));
						cursor.continue();
					} else {
						db.close();
						resolve(keys);
					}
				};
				request.onerror = () => {
					db.close();
					resolve(keys);
				};
			})
			.catch(() => resolve([]));
	});
}

// ---- cloud ----------------------------------------------------------------

// The backend stores every save zstd-compressed (level 19), the same way the
// v86 floppies travel; `fzstd` is already in the bundle for those. Without an
// output buffer it returns an exactly-sized array, which is what a save of
// unknown size wants.
function zstdDecompress(compressed) {
	return decompress(compressed);
}

function cloudUrl(slug) {
	return `/api/games/s/${encodeURIComponent(slug)}/saves`;
}

async function cloudList(slug) {
	const res = await fetch(cloudUrl(slug), { headers: { Authorization: auth() } });
	if (!res.ok) throw new Error(`cloud saves: HTTP ${res.status}`);
	const data = await res.json();
	return data.saves ?? [];
}

async function cloudGet(slug, key) {
	const res = await fetch(`${cloudUrl(slug)}/${encodeURIComponent(key)}`, {
		headers: { Authorization: auth() }
	});
	if (res.status === 404) return null;
	if (!res.ok) throw new Error(`cloud save: HTTP ${res.status}`);
	return zstdDecompress(new Uint8Array(await res.arrayBuffer()));
}

async function cloudPut(slug, key, bytes) {
	const res = await fetch(`${cloudUrl(slug)}/${encodeURIComponent(key)}`, {
		method: 'PUT',
		headers: { Authorization: auth(), 'Content-Type': 'application/octet-stream' },
		body: bytes
	});
	if (!res.ok) throw new Error(await res.text());
}

async function cloudDelete(slug, key) {
	const res = await fetch(`${cloudUrl(slug)}/${encodeURIComponent(key)}`, {
		method: 'DELETE',
		headers: { Authorization: auth() }
	});
	if (!res.ok && res.status !== 404) throw new Error(`cloud save delete: HTTP ${res.status}`);
}

// ---- public surface ---------------------------------------------------------

/**
 * The save-file names a game has, as far as this browser and this account know.
 * A signed-in reader gets the union of the cloud set and the local set; a guest
 * gets the local set. Names only — load the bytes per file.
 */
export async function listSaves(slug) {
	if (!slug) return [];
	const names = new Set(await idbKeys(slug));
	if (auth()) {
		try {
			const cloud = await cloudList(slug);
			for (const entry of cloud) names.add(entry.key);
		} catch {
			// Offline, expired session: the local set is still the truth for this
			// browser, and the game plays with it.
		}
	}
	return [...names];
}

/**
 * One save file's bytes: the account's copy when there is one, this browser's
 * copy otherwise. A corrupt or missing cloud file falls back to local, which
 * is how a save written offline survives a login.
 */
export async function loadSaveFile(slug, key) {
	if (!slug || !key) return null;
	if (auth()) {
		try {
			const bytes = await cloudGet(slug, key);
			if (bytes) return bytes;
		} catch {
			// Fall through to the local copy.
		}
	}
	return idbGet(slug, key);
}

/**
 * Persists one save file. The local copy is always written; a signed-in
 * reader's cloud copy is written too, and a cloud failure leaves the local one
 * standing — the next save retries the mirror.
 */
export async function saveSaveFile(slug, key, bytes) {
	if (!slug || !key || !bytes) return;
	await idbPut(slug, key, bytes);
	if (auth()) {
		try {
			await cloudPut(slug, key, bytes);
		} catch (error) {
			console.warn(`cloud save for ${slug}/${key} failed; kept locally:`, error);
		}
	}
}

/** Removes one save file from both the browser and the account. */
export async function deleteSaveFile(slug, key) {
	if (!slug || !key) return;
	await idbDelete(slug, key);
	if (auth()) {
		try {
			await cloudDelete(slug, key);
		} catch {
			// The cloud row may not exist; local cleanup still applies.
		}
	}
}
