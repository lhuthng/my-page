export class DiskBuildError extends Error {}

const BLOCK = 512;
const PARTITION_START_SECTOR = 63;
const ROOT_ENTRIES = 512;
const ROOT_SECTORS = (ROOT_ENTRIES * 32) / BLOCK;
const RESERVED_SECTORS = 1;
const FAT_COUNT = 2;

const VALID_8_3 = new Set("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789$%'\-_@~`!(){}^#&".split(''));

/**
 * Total byte size (rounded up to a whole sector) for a FAT image that
 * comfortably holds `payloadBytes`, leaving 50% slack for save files.
 * Matches the server's `fat_image_size`.
 */
export function fatImageSize(payloadBytes) {
	const slack = Math.floor(payloadBytes / 2);
	let raw = payloadBytes + slack + 8 * 1024 * 1024;
	raw = Math.min(raw, 1536 * 1024 * 1024);
	return Math.ceil(raw / BLOCK) * BLOCK;
}

/**
 * Cluster size / FAT width, mirroring the reference mtools 4.0.49 behavior
 * for a partitioned disk (`mformat image@@32256`):
 *  - partition <= 16396 sectors (~8MiB): FAT12, 4 sectors/cluster
 *  - partition <= 32736 sectors (~16MiB): FAT12, 8 sectors/cluster
 *  - larger: FAT16, cluster size doubled from 1 until it can hold the
 *    remaining sectors within 65524 clusters (cap 64 sectors/cluster).
 */
function chooseCluster(partSectors) {
	if (partSectors <= 16396) return { spc: 4, fatBits: 12 };
	if (partSectors <= 32736) return { spc: 8, fatBits: 12 };
	const rem = partSectors - 33;
	let spc = 1;
	while (spc * 65524 + 512 < rem) spc <<= 1;
	return { spc, fatBits: 16 };
}

function fatBytesFor(clusters, fatBits) {
	return fatBits === 12 ? Math.ceil((clusters * 12) / 8) : clusters * 2;
}

/**
 * Iterates the FAT size to a fixpoint: data sectors shrink as the FAT grows,
 * so the FAT must cover the clusters that remain. Matches mtools exactly.
 */
function partitionGeometry(diskSize, spc, fatBits) {
	const totalSectors = diskSize / BLOCK;
	const partSectors = totalSectors - PARTITION_START_SECTOR;
	let fatSectors = 1;
	for (let i = 0; i < 64; i++) {
		const dataSectors = partSectors - RESERVED_SECTORS - FAT_COUNT * fatSectors - ROOT_SECTORS;
		if (dataSectors <= 0) {
			throw new DiskBuildError('Game disk partition is too small.');
		}
		const clusters = Math.floor(dataSectors / spc);
		const next = Math.ceil(fatBytesFor(clusters, fatBits) / BLOCK);
		if (next === fatSectors) {
			return {
				partSectors,
				reservedSectors: RESERVED_SECTORS,
				fats: FAT_COUNT,
				rootSectors: ROOT_SECTORS,
				fatSectors,
				clusters,
				fatBits,
				spc,
				dataStartSector: RESERVED_SECTORS + FAT_COUNT * fatSectors + ROOT_SECTORS
			};
		}
		fatSectors = next;
	}
	throw new DiskBuildError('FAT geometry did not converge.');
}

function isReservedWindowsName(component) {
	const stem = component.split('.')[0].replace(/ +$/g, '').toUpperCase();
	if (['CON', 'PRN', 'AUX', 'NUL'].includes(stem)) return true;
	return (
		stem.length === 4 &&
		(stem.startsWith('COM') || stem.startsWith('LPT')) &&
		stem.charCodeAt(3) >= 0x31 &&
		stem.charCodeAt(3) <= 0x39
	);
}

function mangleName(name) {
	const dot = name.lastIndexOf('.');
	let base = dot > 0 ? name.slice(0, dot) : name;
	let ext = dot > 0 ? name.slice(dot + 1) : '';
	base = base.replace(/[. ]+$/g, '');
	ext = ext.replace(/[. ]+$/g, '');
	let dropped = 0;
	const vbase = Array.from(base.toUpperCase())
		.filter((c) => {
			const ok = VALID_8_3.has(c);
			if (!ok) dropped++;
			return ok;
		})
		.join('');
	const vext = Array.from(ext.toUpperCase())
		.filter((c) => {
			const ok = VALID_8_3.has(c);
			if (!ok) dropped++;
			return ok;
		})
		.join('');
	const hasUpper = /[A-Z]/.test(name);
	const hasLower = /[a-z]/.test(name);
	const mixedCase = hasUpper && hasLower;
	const reserved = isReservedWindowsName(base);
	const mangled = dropped > 0 || base.length > 8 || ext.length > 3 || reserved;
	return { vbase, vext, mangled, mixedCase };
}

/** Produces the 11-byte 8.3 short name, honoring uniqueness within a dir. */
function shortName11(name, used) {
	const { vbase, vext, mangled } = mangleName(name);
	let short;
	if (mangled) {
		let n = 1;
		const base6 = vbase.slice(0, 6);
		do {
			const withTilde = `${base6}~${n}`.slice(0, 8);
			short = withTilde.padEnd(8, ' ') + vext.slice(0, 3).padEnd(3, ' ');
			n++;
		} while (used.has(short));
	} else {
		short = vbase.padEnd(8, ' ').slice(0, 8) + vext.padEnd(3, ' ').slice(0, 3);
	}
	used.add(short);
	return short;
}

/** mtools writes an LFN when the name is mixed-case or needs mangling. */
function needsLfn(name) {
	const { mangled, mixedCase } = mangleName(name);
	return mangled || mixedCase;
}

function lfnChecksum(name11) {
	let sum = 0;
	for (let i = 0; i < 11; i++) {
		const b = name11.charCodeAt(i);
		sum = ((sum & 1) << 7) + (sum >> 1) + b;
	}
	return sum & 0xff;
}

const LFN_SLOTS = [1, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 26, 28, 30];

/** LFN entries for a name, ordered first-write-first (0x40|n first). */
function lfnEntries(name, checksum) {
	const chars = Array.from(name);
	const count = Math.ceil(chars.length / 13);
	const entries = [];
	for (let seq = count; seq >= 1; seq--) {
		const order = seq === count ? 0x40 | seq : seq;
		const entry = new Uint8Array(32);
		entry[0] = order;
		entry[11] = 0x0f;
		entry[13] = checksum;
		const startIdx = (count - seq) * 13;
		let terminated = false;
		for (let k = 0; k < 13; k++) {
			const idx = startIdx + k;
			let code = 0xffff;
			if (idx < chars.length) {
				code = chars[idx].charCodeAt(0);
			} else if (idx === chars.length && !terminated) {
				code = 0x0000;
				terminated = true;
			}
			entry[LFN_SLOTS[k]] = code & 0xff;
			entry[LFN_SLOTS[k] + 1] = code >> 8;
		}
		entries.push(entry);
	}
	return entries;
}

function dosDateTime(date) {
	const t = (date.getHours() << 11) | (date.getMinutes() << 5) | (date.getSeconds() >> 1);
	const d =
		(((date.getFullYear() - 1980) & 0x7f) << 9) | ((date.getMonth() + 1) << 5) | date.getDate();
	return { t, d };
}

function shortDirEntry(name11, attr, cluster, size, date) {
	const entry = new Uint8Array(32);
	for (let i = 0; i < 11; i++) entry[i] = name11.charCodeAt(i) & 0xff;
	entry[11] = attr;
	const { t, d } = dosDateTime(date);
	entry[22] = t & 0xff;
	entry[23] = t >> 8;
	entry[24] = d & 0xff;
	entry[25] = d >> 8;
	entry[26] = cluster & 0xff;
	entry[27] = cluster >> 8;
	entry[28] = size & 0xff;
	entry[29] = (size >> 8) & 0xff;
	entry[30] = (size >> 16) & 0xff;
	entry[31] = (size >> 24) & 0xff;
	return entry;
}

function byteCompare(a, b) {
	return a < b ? -1 : a > b ? 1 : 0;
}

function sortByName(a, b) {
	return byteCompare(a.name, b.name);
}

/**
 * Builds a partitioned FAT16/FAT12 disk image containing `files`
 * (`[{ path, data }]`) at the drive root. Returns a sparse image:
 * `{ size, segments: Map<byteOffset, Uint8Array>, geometry }`.
 */
export function buildFatDisk(files, { now = new Date() } = {}) {
	if (!Array.isArray(files) || files.length === 0) {
		throw new DiskBuildError('The game contains no files.');
	}
	let payload = 0;
	for (const file of files) {
		if (!file.data || file.data.length === 0) {
			throw new DiskBuildError(`The game file ${file.path} is empty.`);
		}
		payload += file.data.length;
	}
	const diskSize = fatImageSize(payload);
	const { spc, fatBits } = chooseCluster(diskSize / BLOCK - PARTITION_START_SECTOR);
	const geo = partitionGeometry(diskSize, spc, fatBits);
	const clusterBytes = spc * BLOCK;

	// Build the directory tree.
	const root = { name: '', children: new Map(), parent: null, isDir: true, isRoot: true };
	for (const file of files) {
		const parts = file.path.split('/');
		let node = root;
		for (const part of parts.slice(0, -1)) {
			if (part.length === 0) throw new DiskBuildError(`Invalid game path: ${file.path}`);
			let child = node.children.get(part);
			if (!child) {
				child = { name: part, children: new Map(), parent: node, isDir: true };
				node.children.set(part, child);
			}
			node = child;
		}
		const base = parts[parts.length - 1];
		if (node.children.has(base)) {
			throw new DiskBuildError(`Duplicate path in game tree: ${file.path}`);
		}
		node.children.set(base, { name: base, data: file.data, parent: node, isDir: false });
	}

	const partitionBase = PARTITION_START_SECTOR * BLOCK;
	const dataBase = partitionBase + geo.dataStartSector * BLOCK;

	// Pass 1: short names + LFN layout per directory.
	function planDir(node) {
		const children = [...node.children.values()];
		for (const child of children) {
			child.short = null;
		}
		const used = new Set();
		const entries = [];
		for (const child of children) {
			if (child.isDir) planDir(child);
		}
		for (const child of children) {
			child.short = shortName11(child.name, used);
			const checksum = lfnChecksum(child.short);
			child.lfnBytes = needsLfn(child.name)
				? lfnEntries(child.name, checksum).reduce((n, e) => n + e.length, 0)
				: 0;
			entries.push(child);
		}
		node.plannedEntries = entries;
		node.dirBytes =
			(node.isRoot ? 0 : 64) + entries.reduce((n, child) => n + 32 + child.lfnBytes, 0);
		return node;
	}
	planDir(root);

	// Pass 2: allocate clusters in DFS order (dir's own cluster first, then its
	// children), matching mtools so root dir -> SUBDIR=2 -> data.txt=3.
	let nextCluster = 2;
	function allocate(count) {
		const start = nextCluster;
		nextCluster += count;
		return start;
	}

	const fatEntries = new Map();

	function placeDir(node, parentCluster) {
		const clusterCount = Math.ceil(node.dirBytes / clusterBytes);
		node.selfCluster = allocate(clusterCount);
		const children = node.plannedEntries.slice().sort(sortByName);
		for (const child of children) {
			if (child.isDir) {
				placeDir(child, node.selfCluster);
			} else {
				const count = Math.ceil(child.data.length / clusterBytes);
				child.startCluster = allocate(count);
				child.clusterCount = count;
			}
		}
		// Write the directory content now that children clusters are known.
		const buffer = new Uint8Array(node.dirBytes);
		let offset = 0;
		buffer.set(shortDirEntry('.          ', 0x10, node.selfCluster, 0, now), offset);
		offset += 32;
		buffer.set(shortDirEntry('..         ', 0x10, parentCluster, 0, now), offset);
		offset += 32;
		for (const child of children) {
			if (child.lfnBytes > 0) {
				const checksum = lfnChecksum(child.short);
				for (const entry of lfnEntries(child.name, checksum)) {
					buffer.set(entry, offset);
					offset += 32;
				}
			}
			const attr = child.isDir ? 0x10 : 0x20;
			const cluster = child.isDir ? child.selfCluster : child.startCluster;
			const size = child.isDir ? 0 : child.data.length;
			buffer.set(shortDirEntry(child.short, attr, cluster, size, now), offset);
			offset += 32;
		}
		segments.set(dataBase + (node.selfCluster - 2) * clusterBytes, buffer);
	}

	const segments = new Map();
	{
		const rootContent = new Uint8Array(root.dirBytes);
		let offset = 0;
		const children = root.plannedEntries.slice().sort(sortByName);
		for (const child of children) {
			if (child.isDir) {
				placeDir(child, 0);
			} else {
				const count = Math.ceil(child.data.length / clusterBytes);
				child.startCluster = allocate(count);
				child.clusterCount = count;
			}
		}
		for (const child of children) {
			if (child.lfnBytes > 0) {
				const checksum = lfnChecksum(child.short);
				for (const entry of lfnEntries(child.name, checksum)) {
					rootContent.set(entry, offset);
					offset += 32;
				}
			}
			const attr = child.isDir ? 0x10 : 0x20;
			const cluster = child.isDir ? child.selfCluster : child.startCluster;
			const size = child.isDir ? 0 : child.data.length;
			rootContent.set(shortDirEntry(child.short, attr, cluster, size, now), offset);
			offset += 32;
		}
		segments.set(
			partitionBase + (geo.reservedSectors + geo.fats * geo.fatSectors) * BLOCK,
			rootContent
		);
	}

	// File data segments.
	function placeFiles(node) {
		for (const child of node.plannedEntries) {
			if (child.isDir) {
				placeFiles(child);
			} else {
				segments.set(dataBase + (child.startCluster - 2) * clusterBytes, child.data);
			}
		}
	}
	placeFiles(root);

	// FAT chains.
	function chain(start, count) {
		const clusters = [];
		for (let i = 0; i < count; i++) clusters.push(start + i);
		for (let i = 0; i < clusters.length - 1; i++) fatEntries.set(clusters[i], clusters[i + 1]);
		fatEntries.set(clusters[clusters.length - 1], 0xffff);
	}
	function fatChains(node) {
		if (node.isDir && node !== root) {
			chain(node.selfCluster, Math.ceil(node.dirBytes / clusterBytes));
		}
		for (const child of node.plannedEntries) {
			if (child.isDir) {
				fatChains(child);
			} else {
				chain(child.startCluster, child.clusterCount);
			}
		}
	}
	fatChains(root);

	// FAT image.
	const fatBytes = geo.fatSectors * BLOCK;
	const fat1 = new Uint8Array(fatBytes);
	const fat2 = new Uint8Array(fatBytes);
	if (fatBits === 16) {
		const write16 = (fat, cluster, value) => {
			fat[cluster * 2] = value & 0xff;
			fat[cluster * 2 + 1] = value >> 8;
		};
		write16(fat1, 0, 0xfff8);
		write16(fat1, 1, 0xffff);
		for (const [cluster, next] of fatEntries) write16(fat1, cluster, next);
	} else {
		const write12 = (fat, cluster, value) => {
			const off = Math.floor((cluster * 3) / 2);
			if (cluster % 2 === 0) {
				fat[off] = value & 0xff;
				fat[off + 1] = (fat[off + 1] & 0xf0) | ((value >> 8) & 0x0f);
			} else {
				fat[off] = (fat[off] & 0x0f) | ((value & 0x0f) << 4);
				fat[off + 1] = (value >> 4) & 0xff;
			}
		};
		write12(fat1, 0, 0xff8);
		write12(fat1, 1, 0xfff);
		for (const [cluster, next] of fatEntries) write12(fat1, cluster, next);
	}
	fat2.set(fat1);
	const fatOffset = partitionBase + geo.reservedSectors * BLOCK;
	segments.set(fatOffset, fat1);
	segments.set(fatOffset + geo.fatSectors * BLOCK, fat2);

	// Boot sector.
	const boot = new Uint8Array(512);
	boot.set([0xeb, 0x3c, 0x90]);
	boot.set([0x4d, 0x54, 0x4f, 0x4f, 0x34, 0x30, 0x34, 0x39], 3); // MTOO4049
	boot[11] = BLOCK & 0xff;
	boot[12] = BLOCK >> 8;
	boot[13] = spc;
	boot[14] = RESERVED_SECTORS;
	boot[16] = FAT_COUNT;
	boot[17] = ROOT_ENTRIES & 0xff;
	boot[18] = ROOT_ENTRIES >> 8;
	boot[21] = 0xf8;
	boot[22] = geo.fatSectors & 0xff;
	boot[23] = geo.fatSectors >> 8;
	boot[24] = 63 & 0xff;
	boot[25] = 63 >> 8;
	boot[26] = 16; // heads
	boot[36] = 0x80; // drive number
	boot[38] = 0x29; // extended boot signature
	boot[43] = 0x4e; // 'NO NAME    ' label
	for (let i = 0; i < 11; i++) boot[43 + i] = 'NO NAME    '.charCodeAt(i);
	const fsString = fatBits === 16 ? 'FAT16   ' : 'FAT12   ';
	for (let i = 0; i < 8; i++) boot[54 + i] = fsString.charCodeAt(i);
	if (geo.partSectors <= 0xffff) {
		boot[19] = geo.partSectors & 0xff;
		boot[20] = geo.partSectors >> 8;
	} else {
		const v = geo.partSectors;
		boot[32] = v & 0xff;
		boot[33] = (v >> 8) & 0xff;
		boot[34] = (v >> 16) & 0xff;
		boot[35] = (v >>> 24) & 0xff;
	}
	boot[510] = 0x55;
	boot[511] = 0xaa;
	segments.set(partitionBase, boot);

	// MBR partition entry + signature.
	const endLba = PARTITION_START_SECTOR + geo.partSectors - 1;
	const heads = 255;
	const spt = 63;
	const endCyl = Math.floor(endLba / (heads * spt));
	const remainder = endLba % (heads * spt);
	const endHead = Math.floor(remainder / spt);
	const endSector = (remainder % spt) + 1;
	const entry = new Uint8Array(16);
	entry[0] = 0x80;
	entry[2] = 1;
	entry[4] = 0x06;
	entry[5] = endHead & 0xff;
	entry[6] = endSector & 0xff;
	entry[7] = endCyl & 0xff;
	const start = 63;
	entry[8] = start & 0xff;
	entry[9] = (start >> 8) & 0xff;
	entry[10] = (start >> 16) & 0xff;
	entry[11] = (start >>> 24) & 0xff;
	const partSectors = geo.partSectors;
	entry[12] = partSectors & 0xff;
	entry[13] = (partSectors >> 8) & 0xff;
	entry[14] = (partSectors >> 16) & 0xff;
	entry[15] = (partSectors >>> 24) & 0xff;
	segments.set(446, entry);
	segments.set(510, new Uint8Array([0x55, 0xaa]));

	return { size: diskSize, segments, geometry: geo };
}

export function createEmptyFatDisk(diskSize) {
	if (diskSize < 8 * 1024 * 1024) throw new DiskBuildError('HDD must be at least 8 MB.');
	if (diskSize % BLOCK !== 0) diskSize = Math.ceil(diskSize / BLOCK) * BLOCK;
	const totalSectors = diskSize / BLOCK;
	const partSectors = totalSectors - PARTITION_START_SECTOR;
	const { spc, fatBits } = chooseCluster(partSectors);
	const geo = partitionGeometry(diskSize, spc, fatBits);
	const partitionBase = PARTITION_START_SECTOR * BLOCK;
	const segments = new Map();

	// Boot sector for the partition (empty FAT)
	const boot = new Uint8Array(512);
	boot.set([0xeb, 0x3c, 0x90]);
	boot.set([0x4d, 0x54, 0x4f, 0x4f, 0x34, 0x30, 0x34, 0x39], 3);
	boot[11] = BLOCK & 0xff;
	boot[12] = BLOCK >> 8;
	boot[13] = spc;
	boot[14] = RESERVED_SECTORS;
	boot[16] = FAT_COUNT;
	boot[17] = ROOT_ENTRIES & 0xff;
	boot[18] = ROOT_ENTRIES >> 8;
	boot[21] = 0xf8;
	boot[22] = geo.fatSectors & 0xff;
	boot[23] = geo.fatSectors >> 8;
	boot[24] = 63 & 0xff;
	boot[25] = 63 >> 8;
	boot[26] = 16;
	boot[36] = 0x80;
	boot[38] = 0x29;
	for (let i = 0; i < 11; i++) boot[43 + i] = 'NO NAME    '.charCodeAt(i);
	const fsString = fatBits === 16 ? 'FAT16   ' : 'FAT12   ';
	for (let i = 0; i < 8; i++) boot[54 + i] = fsString.charCodeAt(i);
	if (geo.partSectors <= 0xffff) {
		boot[19] = geo.partSectors & 0xff;
		boot[20] = geo.partSectors >> 8;
	} else {
		const v = geo.partSectors;
		boot[32] = v & 0xff;
		boot[33] = (v >> 8) & 0xff;
		boot[34] = (v >> 16) & 0xff;
		boot[35] = (v >>> 24) & 0xff;
	}
	boot[510] = 0x55;
	boot[511] = 0xaa;
	segments.set(partitionBase, boot);

	// FATs (empty, only media + EOF)
	const fatBytes = geo.fatSectors * BLOCK;
	const fat1 = new Uint8Array(fatBytes);
	const fat2 = new Uint8Array(fatBytes);
	if (fatBits === 16) {
		fat1[0] = 0xf8; fat1[1] = 0xff; fat1[2] = 0xff; fat1[3] = 0xff;
		fat2[0] = 0xf8; fat2[1] = 0xff; fat2[2] = 0xff; fat2[3] = 0xff;
	} else {
		fat1[0] = 0xf8; fat1[1] = 0xff; fat1[2] = 0xff;
		fat2[0] = 0xf8; fat2[1] = 0xff; fat2[2] = 0xff;
	}
	const fatOffset = partitionBase + RESERVED_SECTORS * BLOCK;
	segments.set(fatOffset, fat1);
	segments.set(fatOffset + geo.fatSectors * BLOCK, fat2);

	// Root directory (empty, just . and .. will be created on first write, but we init empty)
	const rootOffset = partitionBase + (RESERVED_SECTORS + FAT_COUNT * geo.fatSectors) * BLOCK;
	segments.set(rootOffset, new Uint8Array(ROOT_SECTORS * BLOCK));

	// MBR
	const endLba = PARTITION_START_SECTOR + geo.partSectors - 1;
	const heads = 255; const spt = 63;
	const endCyl = Math.floor(endLba / (heads * spt));
	const remainder = endLba % (heads * spt);
	const endHead = Math.floor(remainder / spt);
	const endSector = (remainder % spt) + 1;
	const entry = new Uint8Array(16);
	entry[0] = 0x80; entry[2] = 1; entry[4] = 0x06;
	entry[5] = endHead & 0xff; entry[6] = endSector & 0xff; entry[7] = endCyl & 0xff;
	const start = 63;
	entry[8] = start & 0xff; entry[9] = (start >> 8) & 0xff; entry[10] = (start >> 16) & 0xff; entry[11] = (start >>> 24) & 0xff;
	entry[12] = geo.partSectors & 0xff; entry[13] = (geo.partSectors >> 8) & 0xff; entry[14] = (geo.partSectors >> 16) & 0xff; entry[15] = (geo.partSectors >>> 24) & 0xff;
	segments.set(446, entry);
	segments.set(510, new Uint8Array([0x55, 0xaa]));

	// Materialize to flat buffer for v86 (which expects a flat hda/hdb url, not sparse parts)
	const flat = new Uint8Array(diskSize);
	for (const [off, seg] of segments) {
		flat.set(seg, off);
	}
	return { size: diskSize, segments, flat, geometry: geo };
}

export function readHddFiles(image) {
	if (!(image instanceof Uint8Array) || image.length < 512) return [];
	// MBR partition entry at 446, check if it looks like a valid FAT partition
	const partType = image[450 + 4];
	// Accept 0x06 (FAT16), 0x0B/0x0C (FAT32), 0xE (FAT16 LBA) or 0x04/0x01 (FAT16/12) for our disks
	const validTypes = new Set([0x01, 0x04, 0x06, 0x0b, 0x0c, 0x0e]);
	if (!validTypes.has(partType)) {
		// Try floppy-style directly (no partition) as fallback
		return [];
	}
	const partStart = (image[454] | (image[455] << 8) | (image[456] << 16) | (image[457] << 24) >>> 0) * 512;
	if (partStart === 0 || partStart >= image.length) return [];
	const partBytes = image.subarray(partStart);
	if (partBytes.length < 512) return [];
	const view = new DataView(partBytes.buffer, partBytes.byteOffset, partBytes.byteLength);
	const bytesPerSector = view.getUint16(11, true);
	if (bytesPerSector !== 512) return [];
	// Reuse floppy reader logic but with partition offset
	const sectorsPerCluster = partBytes[13];
	const reservedSectors = view.getUint16(14, true);
	const fatCount = partBytes[16];
	const rootEntries = view.getUint16(17, true);
	let totalSectors = view.getUint16(19, true);
	if (totalSectors === 0) totalSectors = view.getUint32(32, true);
	const fatSectors = view.getUint16(22, true);
	const rootStart = (reservedSectors + fatCount * fatSectors) * bytesPerSector;
	const rootBytes = rootEntries * 32;
	const dataStart = rootStart + rootBytes;
	const clusterBytes = sectorsPerCluster * bytesPerSector;
	if (clusterBytes <= 0) return [];
	// Estimate fatBits from rootEntries/fat size similar to floppy
	const dataSectors = totalSectors - reservedSectors - fatCount * fatSectors - rootEntries * 32 / bytesPerSector;
	const clusters = Math.floor(dataSectors / sectorsPerCluster);
	const fatBits = clusters < 4085 ? 12 : 16;
	const fatStart = reservedSectors * bytesPerSector;

	const u16 = (off) => view.getUint16(off, true);
	const nextCluster = (n) => {
		if (fatBits === 12) {
			const off = fatStart + n + (n >> 1);
			const lo = partBytes[off];
			const hi = partBytes[off + 1];
			return (n & 1) === 0 ? lo | ((hi & 0x0f) << 8) : (lo >> 4) | (hi << 4);
		}
		return u16(fatStart + n * 2);
	};
	const readChain = (start) => {
		const parts = [];
		let n = start;
		while (n >= 2 && n < 0xfff8 && n < clusters + 2) {
			const off = dataStart + (n - 2) * clusterBytes;
			if (off + clusterBytes > partBytes.length) break;
			parts.push(partBytes.subarray(off, off + clusterBytes));
			n = nextCluster(n);
		}
		if (parts.length === 0) return new Uint8Array(0);
		const total = parts.reduce((s, p) => s + p.length, 0);
		const out = new Uint8Array(total);
		let o = 0;
		for (const p of parts) { out.set(p, o); o += p.length; }
		return out;
	};
	const readName = (bytes) => String.fromCharCode(...bytes).replace(/[^\x20-\x7e]+/g, '').trim();
	const files = [];
	const readDir = (dirBytes, prefix) => {
		const dirView = new DataView(dirBytes.buffer, dirBytes.byteOffset, dirBytes.byteLength);
		for (let i = 0; i + 32 <= dirBytes.length; i += 32) {
			if (dirBytes[i] === 0) break;
			if (dirBytes[i] === 0xe5) continue;
			const attr = dirBytes[i + 11];
			if (attr === 0x0f) continue;
			const short = (() => {
				const base = readName(dirBytes.subarray(i, i + 8));
				const ext = readName(dirBytes.subarray(i + 8, i + 11));
				return ext ? `${base}.${ext}` : base;
			})();
			if (!short || short === '.' || short === '..') continue;
			const firstCluster = dirView.getUint16(i + 26, true);
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
		readDir(partBytes.subarray(rootStart, rootStart + rootBytes), '');
	} catch { return []; }
	return files;
}

/**
 * Materializes one chunk of a sparse image into a chunk-sized buffer (the
 * last chunk is zero-padded to exactly `chunkSize`, matching the server's
 * `split_asset` so v86's `use_parts` offsets line up). Chunks are independent,
 * so parallel workers can build them in any order.
 */
export function sparseChunkAt(sparse, chunkSize, chunkIndex) {
	const { size, segments } = sparse;
	const start = chunkIndex * chunkSize;
	const end = Math.min(start + chunkSize, size);
	const buffer = new Uint8Array(chunkSize);
	for (const offset of [...segments.keys()].sort((a, b) => a - b)) {
		if (offset >= end) break;
		const seg = segments.get(offset);
		if (offset + seg.length <= start) continue;
		const copyFrom = Math.max(0, start - offset);
		const copyLen = Math.min(seg.length - copyFrom, end - offset - copyFrom);
		if (copyLen > 0) {
			buffer.set(seg.subarray(copyFrom, copyFrom + copyLen), offset + copyFrom - start);
		}
	}
	return buffer;
}

/**
 * Materializes a sparse image into chunk-sized buffers, one standalone zstd
 * frame per chunk. The last chunk is zero-padded to exactly `chunkSize`,
 * matching the server's `split_asset` so v86's `use_parts` offsets line up.
 */
export function* sparseChunks(sparse, chunkSize) {
	const { size } = sparse;
	const totalChunks = Math.ceil(size / chunkSize);
	for (let chunkIndex = 0; chunkIndex < totalChunks; chunkIndex++) {
		yield sparseChunkAt(sparse, chunkSize, chunkIndex);
	}
}
