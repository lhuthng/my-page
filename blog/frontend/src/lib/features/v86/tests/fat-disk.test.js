import { test } from 'node:test';
import assert from 'node:assert/strict';
import { buildFatDisk } from '../fat-disk.js';

const LFN_SLOTS = [1, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30];

// Builds a real image with long names (including a 13-char one, which exactly
// fills one LFN entry) and re-validates from the raw bytes what Windows
// validates: every LFN checksum matches its short name, and the entries
// reassemble to the original name with a 0x0000 terminator. A wrong checksum
// or a missing terminator makes Windows ignore the LFN, so long filenames
// silently vanish and the game fails to find its own files.
test('LFN entries match short names and reassemble', () => {
	const enc = new TextEncoder();
	const files = [
		{ path: 'Doraemon-en.exe', data: enc.encode('x'.repeat(100)) },
		{ path: 'bitmaps-en.dat', data: enc.encode('y'.repeat(100)) },
		{ path: 'interface.dat', data: enc.encode('z'.repeat(100)) },
		{ path: 'MGame00.DAT', data: enc.encode('w'.repeat(100)) }
	];
	const sparse = buildFatDisk(files, { now: new Date('2000-01-01T00:00:00Z') });
	const img = new Uint8Array(sparse.size);
	for (const [off, seg] of sparse.segments) img.set(seg, off);

	const view = new DataView(img.buffer);
	const base = 63 * 512; // partition start
	const bps = view.getUint16(base + 11, true);
	const reserved = view.getUint16(base + 14, true);
	const fats = img[base + 16];
	const fatSectors = view.getUint16(base + 22, true);
	let at = base + (reserved + fats * fatSectors) * bps;

	const ref = (bytes) => {
		let sum = 0;
		for (const b of bytes) sum = (((sum & 1) << 7) + (sum >> 1) + b) & 0xff;
		return sum;
	};

	const found = new Map();
	let pending = [];
	for (; ; at += 32) {
		const first = img[at];
		if (first === 0x00) break;
		if (first === 0xe5) continue;
		if (img[at + 11] === 0x0f) {
			pending.push(at);
			continue;
		}
		const short = img.slice(at, at + 11);
		const expected = ref(short);
		assert.ok(pending.length > 0, 'long name without LFN entries');
		for (const off of pending) assert.equal(img[off + 13], expected, 'LFN checksum mismatch');
		// VFAT positions chunk `s` at chars[(s-1)*13 …] regardless of physical
		// order: reassemble the way Windows does, and require the terminator
		// so names that exactly fill their entries (13, 26, … chars) still
		// resolve. Assembling in physical order instead would mirror a
		// swapped-chunk builder bug without catching it.
		const chunks = new Map();
		let terminated = false;
		for (const off of pending) {
			chunks.set(img[off] & 0x1f, off);
			for (const slot of LFN_SLOTS) {
				if ((img[off + slot] | (img[off + slot + 1] << 8)) === 0x0000) terminated = true;
			}
		}
		assert.ok(terminated, 'LFN name missing terminator');
		let chars = [];
		for (const [seq, off] of [...chunks].sort((a, b) => a[0] - b[0])) {
			const frag = [];
			for (const slot of LFN_SLOTS) {
				const code = img[off + slot] | (img[off + slot + 1] << 8);
				if (code === 0x0000) break;
				if (code === 0xffff) continue;
				frag.push(String.fromCharCode(code));
			}
			while (chars.length < (seq - 1) * 13) chars.push('');
			for (const c of frag) chars.push(c);
		}
		const name = chars.join('').split('\0')[0];
		const shortText = String.fromCharCode(...short).trim();
		found.set(name, shortText);
		pending = [];
	}
	assert.equal(found.get('Doraemon-en.exe'), 'DORAEM~1EXE');
	assert.equal(found.get('bitmaps-en.dat'), 'BITMAP~1DAT');
	assert.equal(found.get('interface.dat'), 'INTERF~1DAT');
	assert.equal(found.get('MGame00.DAT'), 'MGAME00 DAT');
});
