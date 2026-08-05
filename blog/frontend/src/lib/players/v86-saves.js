import { decompress } from 'fzstd';
import { auth } from '$lib/auth/user.svelte.js';

export const SAVE_BYTES = 1474560;
export const BLANK_FLOPPY_URL = '/v86/floppy.img.zst';

let blankFloppyPromise = null;

/** A recognizable, ready-to-mount floppy is exactly 1.44 MB and has a FAT boot
 *  sector (0x55AA signature, 512-byte sectors). Anything else (e.g. a corrupt
 *  or unformatted image) is treated as "no save". */
function isFormattedFloppy(bytes) {
	if (!(bytes instanceof Uint8Array) || bytes.length !== SAVE_BYTES) return false;
	if (bytes[510] !== 0x55 || bytes[511] !== 0xaa) return false;
	return bytes[11] === 0 && bytes[12] === 0x02;
}

/** Builds a fresh, empty 1.44 MB FAT12 floppy entirely in-memory.
 *  Boot sector + BPB for 80 cyl / 2 heads / 18 spt, media 0xF0, 2 FATs, 224 root dir entries. */
function createBlankFloppy() {
	const bytes = new Uint8Array(SAVE_BYTES);
	const dv = new DataView(bytes.buffer);
	// Boot sector (offset 0)
	bytes[0] = 0xeb;
	bytes[1] = 0x3c;
	bytes[2] = 0x90; // jmp + nop
	bytes.set([0x4d, 0x53, 0x44, 0x4f, 0x53, 0x35, 0x2e, 0x30], 3); // OEM "MSDOS5.0"
	dv.setUint16(11, 512, true); // bytes per sector
	bytes[13] = 1; // sectors per cluster
	dv.setUint16(14, 1, true); // reserved sectors
	bytes[16] = 2; // FAT count
	dv.setUint16(17, 224, true); // root dir entries
	dv.setUint16(19, 2880, true); // total sectors (16-bit, for < 32M)
	bytes[21] = 0xf0; // media descriptor (floppy 1.44M)
	dv.setUint16(22, 9, true); // sectors per FAT
	dv.setUint16(24, 18, true); // sectors per track
	dv.setUint16(26, 2, true); // heads
	dv.setUint32(28, 0, true); // hidden sectors
	dv.setUint32(32, 2880, true); // total sectors (32-bit)
	// Extended BPB (DOS 4.0+)
	bytes[36] = 0x00; // drive number
	bytes[37] = 0x00; // reserved
	bytes[38] = 0x29; // extended boot sig
	dv.setUint32(39, 0x12345678, true); // volume serial
	bytes.set([0x4e, 0x4f, 0x20, 0x4e, 0x41, 0x4d, 0x45, 0x20, 0x20, 0x20, 0x20], 43); // "NO NAME    "
	bytes.set([0x46, 0x41, 0x54, 0x31, 0x32, 0x20, 0x20, 0x20], 54); // "FAT12   "
	// Bootstrap code area (offset 62..509) left zero — not executed by v86
	bytes[510] = 0x55;
	bytes[511] = 0xaa; // boot signature
	// FATs (two copies, starting at sector 1 and sector 10)
	// FAT[0] = media, FAT[1] = end-of-chain for root dir
	const fatStart = 512;
	bytes[fatStart + 0] = 0xf0;
	bytes[fatStart + 1] = 0xff;
	bytes[fatStart + 2] = 0xff; // EOF
	const fatSize = 9 * 512;
	// Second FAT at sector 10
	const fat2Start = fatStart + fatSize;
	bytes[fat2Start + 0] = 0xf0;
	bytes[fat2Start + 1] = 0xff;
	bytes[fat2Start + 2] = 0xff;
	// Root directory at sector 19 (offset 19*512 = 9728) — zeroed by default
	// Boot signature already set above
	return bytes;
}

/** Decompresses the bundled blank floppy once and reuses the buffer. */
export async function loadBlankFloppy() {
	if (!blankFloppyPromise) {
		blankFloppyPromise = (async () => {
			try {
				const res = await fetch(BLANK_FLOPPY_URL);
				if (res.ok) {
					const save = zstdDecompress(new Uint8Array(await res.arrayBuffer()));
					if (isFormattedFloppy(save)) return save;
				}
			} catch (e) {
				console.warn('Blank floppy asset unavailable, using generated floppy:', e);
			}
			// Fallback: generate a valid formatted floppy in-memory
			console.log('Using generated blank floppy');
			return createBlankFloppy();
		})();
	}
	return blankFloppyPromise;
}

const IDB_NAME = 'v86-saves';
const IDB_STORE = 'saves';

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

function idbGet(slug) {
	return new Promise((resolve, reject) => {
		openDb()
			.then((db) => {
				const tx = db.transaction(IDB_STORE, 'readonly');
				const request = tx.objectStore(IDB_STORE).get(slug);
				request.onsuccess = () => {
					const value = request.result;
					db.close();
					if (value instanceof Uint8Array) resolve(value);
					else if (value instanceof ArrayBuffer) resolve(new Uint8Array(value));
					else resolve(null);
				};
				request.onerror = () => {
					db.close();
					reject(request.error);
				};
			})
			.catch(reject);
	});
}

function idbPut(slug, floppy) {
	return new Promise((resolve, reject) => {
		openDb()
			.then((db) => {
				const tx = db.transaction(IDB_STORE, 'readwrite');
				tx.objectStore(IDB_STORE).put(floppy, slug);
				tx.oncomplete = () => {
					db.close();
					resolve();
				};
				tx.onerror = () => {
					db.close();
					reject(tx.error);
				};
			})
			.catch(reject);
	});
}

function idbDelete(slug) {
	return new Promise((resolve, reject) => {
		openDb()
			.then((db) => {
				const tx = db.transaction(IDB_STORE, 'readwrite');
				tx.objectStore(IDB_STORE).delete(slug);
				tx.oncomplete = () => {
					db.close();
					resolve();
				};
				tx.onerror = () => {
					db.close();
					reject(tx.error);
				};
			})
			.catch(reject);
	});
}

function zstdDecompress(compressed) {
	const output = new Uint8Array(SAVE_BYTES);
	decompress(compressed, output);
	return output;
}

async function cloudGet(slug) {
	const res = await fetch(`/api/projects/s/${encodeURIComponent(slug)}/v86/saves`, {
		headers: { Authorization: auth() }
	});
	if (res.status === 404) return null;
	if (!res.ok) throw new Error('Cloud save could not be loaded.');
	return zstdDecompress(new Uint8Array(await res.arrayBuffer()));
}

async function cloudPut(slug, floppy) {
	const res = await fetch(`/api/projects/s/${encodeURIComponent(slug)}/v86/saves`, {
		method: 'PUT',
		headers: {
			Authorization: auth(),
			'Content-Type': 'application/octet-stream'
		},
		body: floppy
	});
	if (!res.ok) throw new Error('Cloud save could not be uploaded.');
}

async function cloudDelete(slug) {
	const res = await fetch(`/api/projects/s/${encodeURIComponent(slug)}/v86/saves`, {
		method: 'DELETE',
		headers: { Authorization: auth() }
	});
	if (!res.ok) throw new Error('Cloud save could not be cleared.');
}

/**
 * Loads the saved floppy image for a game, preferring the account-bound cloud
 * save and falling back to the browser-local snapshot.
 * Returns a Uint8Array floppy image or null when there is no save yet.
 */
export async function loadSave(slug) {
	if (!slug) return null;
	if (auth()) {
		try {
			const save = await cloudGet(slug);
			if (isFormattedFloppy(save)) return save;
		} catch {
			// Offline, backend hiccup, or a corrupt/legacy save: fall back to
			// the local snapshot or, failing that, to the blank floppy.
		}
	}
	const local = await idbGet(slug);
	return isFormattedFloppy(local) ? local : null;
}

/**
 * Persists a floppy image. Logged-in users get a cloud save; guests keep the
 * snapshot in this browser only.
 */
export async function saveGame(slug, floppy) {
	if (!slug || !floppy) return;
	if (auth()) {
		await cloudPut(slug, floppy);
	} else {
		await idbPut(slug, floppy);
	}
}

/**
 * Removes the save for a game in both locations.
 */
export async function clearSave(slug) {
	if (!slug) return;
	if (auth()) {
		try {
			await cloudDelete(slug);
		} catch {
			// The cloud row may not exist yet; local cleanup still applies.
		}
	}
	await idbDelete(slug);
}
