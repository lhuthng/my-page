import { zipSync } from 'fflate';

// Reads the current contents of a FAT12/FAT16 floppy image and packages them
// as a .zip so the sandbox can hand files back out of the guest. This is the
// inverse of the game-disk writer in fat-disk.js: it walks the real on-disk
// directory tree instead of building one.

const u16 = (view, off) => view.getUint16(off, true);

const readName = (bytes) =>
	String.fromCharCode(...bytes)
		.replace(/[^\x20-\x7e]+/g, '')
		.trim();

/** Extracts `[{ name, data }]` (directories flattened into `/` paths) from a
 *  FAT12/FAT16 floppy image. Returns an empty list for an unreadable image. */
export function readFloppyFiles(image) {
	if (!(image instanceof Uint8Array) || image.length < 512) return [];
	const view = new DataView(image.buffer, image.byteOffset, image.byteLength);

	const bytesPerSector = u16(view, 11);
	const sectorsPerCluster = image[13];
	const reservedSectors = u16(view, 14);
	const fatCount = image[16];
	const rootEntries = u16(view, 17);
	let totalSectors = u16(view, 19);
	if (totalSectors === 0) totalSectors = view.getUint32(32, true);
	const fatSectors = u16(view, 22);

	const rootStart = (reservedSectors + fatCount * fatSectors) * bytesPerSector;
	const rootBytes = rootEntries * 32;
	const dataStart = rootStart + rootBytes;
	const clusterBytes = sectorsPerCluster * bytesPerSector;
	const clusterCount = Math.floor((totalSectors * bytesPerSector - dataStart) / clusterBytes);
	if (clusterBytes <= 0 || clusterCount <= 0) return [];
	const fatBits = clusterCount < 4085 ? 12 : 16;
	const fatStart = reservedSectors * bytesPerSector;

	const nextCluster = (n) => {
		if (fatBits === 12) {
			const off = fatStart + n + (n >> 1);
			const lo = image[off];
			const hi = image[off + 1];
			return (n & 1) === 0 ? lo | ((hi & 0x0f) << 8) : (lo >> 4) | (hi << 4);
		}
		return u16(view, fatStart + n * 2);
	};

	const readChain = (start) => {
		const parts = [];
		let n = start;
		while (n >= 2 && n < 0x0ff8 && n < clusterCount + 2) {
			const off = dataStart + (n - 2) * clusterBytes;
			parts.push(image.subarray(off, off + clusterBytes));
			n = nextCluster(n);
		}
		if (parts.length === 0) return new Uint8Array(0);
		const total = parts.reduce((s, p) => s + p.length, 0);
		const out = new Uint8Array(total);
		let o = 0;
		for (const p of parts) {
			out.set(p, o);
			o += p.length;
		}
		return out;
	};

	const files = [];
	const readDir = (dirBytes, prefix) => {
		const dirView = new DataView(dirBytes.buffer, dirBytes.byteOffset, dirBytes.byteLength);
		for (let i = 0; i + 32 <= dirBytes.length; i += 32) {
			if (dirBytes[i] === 0) break; // end of directory
			if (dirBytes[i] === 0xe5) continue; // deleted entry
			const attr = dirBytes[i + 11];
			if (attr === 0x0f) continue; // LFN record, skipped

			const short = (() => {
				const base = readName(dirBytes.subarray(i, i + 8));
				const ext = readName(dirBytes.subarray(i + 8, i + 11));
				return ext ? `${base}.${ext}` : base;
			})();
			if (!short || short === '.' || short === '..') continue;

			const firstCluster = u16(dirView, i + 26);
			const size = dirView.getUint32(i + 28, true);

			if (attr & 0x10) {
				readDir(readChain(firstCluster), `${prefix}${short}/`);
			} else {
				const content = readChain(firstCluster);
				files.push({ name: `${prefix}${short}`, data: content.slice(0, size) });
			}
		}
	};

	try {
		readDir(image.subarray(rootStart, rootStart + rootBytes), '');
	} catch {
		return [];
	}
	return files;
}

/** Packs `[{ name, data }]` into a .zip Uint8Array via fflate. */
export function floppyFilesToZip(files) {
	const data = {};
	for (const file of files) data[file.name] = file.data;
	return zipSync(data);
}
