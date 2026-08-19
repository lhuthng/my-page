// ISO 9660 + Joliet writer. Unlike a hard disk, a CD can be swapped while the
// machine runs (set_cdrom raises medium_changed), which is what lets the
// sandbox replace the game without rebooting Windows.
//
// Two directory hierarchies are emitted over one set of file extents: the
// primary one with DOS-ish uppercase names, and a Joliet one with the original
// names in UCS-2, which is what Windows 9x actually reads.

const SECTOR = 2048;
const ROOT_RECORD_SIZE = 34;
const MAX_IMAGE_BYTES = 2 * 1024 * 1024 * 1024;

const INVALID_ISO = /[^A-Z0-9_]/g;
const ascii = (text) => Uint8Array.from(text, (c) => c.charCodeAt(0) & 0x7f);

const both16 = (view, at, value) => {
	view.setUint16(at, value, true);
	view.setUint16(at + 2, value, false);
};
const both32 = (view, at, value) => {
	view.setUint32(at, value, true);
	view.setUint32(at + 4, value, false);
};

const sectors = (bytes) => Math.ceil(bytes / SECTOR);

/** Identifier length for a directory record, padded to an even total. */
const recordSize = (idLength) => 33 + idLength + (idLength % 2 === 0 ? 1 : 0);

function isoNameFor(node, taken) {
	const name = node.name.toUpperCase();
	let id;
	if (node.isDir) {
		id = (name.replace(/\./g, '_').replace(INVALID_ISO, '_') || 'DIR').slice(0, 31);
	} else {
		const dot = name.lastIndexOf('.');
		const ext = (dot > 0 ? name.slice(dot + 1) : '').replace(INVALID_ISO, '_').slice(0, 3);
		// Level 2 caps the whole identifier at 31 characters, and ";1" is part of it.
		const maxBase = 31 - 2 - (ext ? ext.length + 1 : 0);
		const base =
			(dot > 0 ? name.slice(0, dot) : name).replace(INVALID_ISO, '_').slice(0, maxBase) || '_';
		id = ext ? `${base}.${ext}` : base;
	}
	let candidate = id;
	for (let n = 1; taken.has(candidate); n++) {
		const tail = `~${n}`;
		const dot = id.lastIndexOf('.');
		const base = dot > 0 ? id.slice(0, dot) : id;
		const ext = dot > 0 ? id.slice(dot) : '';
		const maxBase = 31 - 2 - ext.length - tail.length;
		candidate = base.slice(0, Math.max(1, maxBase)) + tail + ext;
	}
	taken.add(candidate);
	return node.isDir ? candidate : `${candidate};1`;
}

function jolietNameFor(node, taken) {
	// Joliet identifiers are UCS-2 and capped at 64 characters.
	let id = node.name.replace(/[*/:;?\\]/g, '_').slice(0, node.isDir ? 64 : 62);
	let candidate = id;
	for (let n = 1; taken.has(candidate.toUpperCase()); n++) {
		const tail = `~${n}`;
		candidate = id.slice(0, Math.max(1, id.length - tail.length)) + tail;
	}
	taken.add(candidate.toUpperCase());
	return node.isDir ? candidate : `${candidate};1`;
}

const identifierBytes = (name, joliet) => {
	if (!joliet) return ascii(name);
	const bytes = new Uint8Array(name.length * 2);
	for (let i = 0; i < name.length; i++) {
		const code = name.charCodeAt(i);
		bytes[i * 2] = code >> 8;
		bytes[i * 2 + 1] = code & 0xff;
	}
	return bytes;
};

/** Directories breadth-first, which is the order the path table requires. */
function orderDirectories(root) {
	const list = [{ node: root, parentIndex: 1 }];
	root.pathIndex = 1;
	for (let i = 0; i < list.length; i++) {
		const { node } = list[i];
		for (const child of node.sorted) {
			if (!child.isDir) continue;
			child.pathIndex = list.length + 1;
			list.push({ node: child, parentIndex: node.pathIndex });
		}
	}
	return list;
}

/** Records are packed into an extent but may never straddle a sector. */
function extentSize(node, joliet) {
	let offset = ROOT_RECORD_SIZE * 2; // "." and ".."
	for (const child of node.sorted) {
		const size = recordSize(
			identifierBytes(joliet ? child.jolietName : child.isoName, joliet).length
		);
		if ((offset % SECTOR) + size > SECTOR) offset += SECTOR - (offset % SECTOR);
		offset += size;
	}
	return Math.max(SECTOR, sectors(offset) * SECTOR);
}

function pathTableSize(directories, joliet) {
	let total = 0;
	for (const { node } of directories) {
		const length =
			node.pathIndex === 1
				? 1
				: identifierBytes(joliet ? node.jolietName : node.isoName, joliet).length;
		total += 8 + length + (length % 2);
	}
	return total;
}

function annotate(node) {
	node.sorted = [...node.children.values()];
	const isoTaken = new Set();
	const jolietTaken = new Set();
	for (const child of node.sorted) {
		child.isoName = isoNameFor(child, isoTaken);
		child.jolietName = jolietNameFor(child, jolietTaken);
	}
	// ISO requires directory records ordered by identifier.
	node.sorted.sort((a, b) => (a.isoName < b.isoName ? -1 : a.isoName > b.isoName ? 1 : 0));
	for (const child of node.sorted) if (child.isDir) annotate(child);
}

export function planIso(root) {
	annotate(root);
	const directories = orderDirectories(root);

	const primaryPathBytes = pathTableSize(directories, false);
	const jolietPathBytes = pathTableSize(directories, true);

	let lba = 19; // 0-15 system area, 16 PVD, 17 SVD, 18 terminator
	const primaryPathL = lba;
	lba += sectors(primaryPathBytes);
	const primaryPathM = lba;
	lba += sectors(primaryPathBytes);
	const jolietPathL = lba;
	lba += sectors(jolietPathBytes);
	const jolietPathM = lba;
	lba += sectors(jolietPathBytes);

	for (const { node } of directories) {
		node.primaryExtent = lba;
		node.primarySize = extentSize(node, false);
		lba += sectors(node.primarySize);
	}
	for (const { node } of directories) {
		node.jolietExtent = lba;
		node.jolietSize = extentSize(node, true);
		lba += sectors(node.jolietSize);
	}

	// One set of file extents, referenced by both hierarchies.
	const files = [];
	const walk = (node) => {
		for (const child of node.sorted) {
			if (child.isDir) walk(child);
			else {
				child.extent = lba;
				lba += sectors(child.size);
				files.push(child);
			}
		}
	};
	walk(root);

	const byteLength = lba * SECTOR;
	if (byteLength > MAX_IMAGE_BYTES) {
		throw new Error('That game is too big for a CD (2 GB max).');
	}
	return {
		directories,
		files,
		primaryPathL,
		primaryPathM,
		jolietPathL,
		jolietPathM,
		primaryPathBytes,
		jolietPathBytes,
		totalSectors: lba,
		byteLength
	};
}

function writeRecordDate(view, at, date) {
	view.setUint8(at, date.getUTCFullYear() - 1900);
	view.setUint8(at + 1, date.getUTCMonth() + 1);
	view.setUint8(at + 2, date.getUTCDate());
	view.setUint8(at + 3, date.getUTCHours());
	view.setUint8(at + 4, date.getUTCMinutes());
	view.setUint8(at + 5, date.getUTCSeconds());
	view.setInt8(at + 6, 0);
}

function writeVolumeDate(image, at, date) {
	const pad = (value, width) => String(value).padStart(width, '0');
	const text =
		`${pad(date.getUTCFullYear(), 4)}${pad(date.getUTCMonth() + 1, 2)}${pad(date.getUTCDate(), 2)}` +
		`${pad(date.getUTCHours(), 2)}${pad(date.getUTCMinutes(), 2)}${pad(date.getUTCSeconds(), 2)}00`;
	image.set(ascii(text), at);
	image[at + 16] = 0;
}

/** One directory record. Returns how many bytes it took. */
function writeRecord(image, view, at, identifier, extent, size, isDir, date) {
	const length = recordSize(identifier.length);
	view.setUint8(at, length);
	view.setUint8(at + 1, 0);
	both32(view, at + 2, extent);
	both32(view, at + 10, size);
	writeRecordDate(view, at + 18, date);
	view.setUint8(at + 25, isDir ? 0x02 : 0x00);
	view.setUint8(at + 26, 0);
	view.setUint8(at + 27, 0);
	both16(view, at + 28, 1);
	view.setUint8(at + 32, identifier.length);
	image.set(identifier, at + 33);
	return length;
}

function writeDirectoryExtent(image, view, node, parent, joliet, date) {
	const base = (joliet ? node.jolietExtent : node.primaryExtent) * SECTOR;
	const selfExtent = joliet ? node.jolietExtent : node.primaryExtent;
	const selfSize = joliet ? node.jolietSize : node.primarySize;
	const parentExtent = joliet ? parent.jolietExtent : parent.primaryExtent;
	const parentSize = joliet ? parent.jolietSize : parent.primarySize;

	let offset = 0;
	offset += writeRecord(image, view, base, Uint8Array.of(0), selfExtent, selfSize, true, date);
	offset += writeRecord(
		image,
		view,
		base + offset,
		Uint8Array.of(1),
		parentExtent,
		parentSize,
		true,
		date
	);

	for (const child of node.sorted) {
		const identifier = identifierBytes(joliet ? child.jolietName : child.isoName, joliet);
		const size = recordSize(identifier.length);
		if ((offset % SECTOR) + size > SECTOR) offset += SECTOR - (offset % SECTOR);
		writeRecord(
			image,
			view,
			base + offset,
			identifier,
			child.isDir ? (joliet ? child.jolietExtent : child.primaryExtent) : child.extent,
			child.isDir ? (joliet ? child.jolietSize : child.primarySize) : child.size,
			child.isDir,
			date
		);
		offset += size;
	}
}

function writePathTable(image, view, at, directories, joliet, littleEndian) {
	let offset = at;
	for (const { node, parentIndex } of directories) {
		const identifier =
			node.pathIndex === 1
				? Uint8Array.of(0)
				: identifierBytes(joliet ? node.jolietName : node.isoName, joliet);
		view.setUint8(offset, identifier.length);
		view.setUint8(offset + 1, 0);
		view.setUint32(offset + 2, joliet ? node.jolietExtent : node.primaryExtent, littleEndian);
		view.setUint16(offset + 6, parentIndex, littleEndian);
		image.set(identifier, offset + 8);
		offset += 8 + identifier.length + (identifier.length % 2);
	}
}

function writeDescriptor(image, view, lba, type, plan, root, joliet, date, label) {
	const at = lba * SECTOR;
	view.setUint8(at, type);
	image.set(ascii('CD001'), at + 1);
	view.setUint8(at + 6, 1);
	image.fill(0x20, at + 8, at + 40); // system identifier

	const identifier = joliet ? identifierBytes(label, true) : ascii(label);
	image.fill(joliet ? 0x00 : 0x20, at + 40, at + 72);
	if (joliet) {
		// Joliet pads its text fields with UCS-2 spaces.
		for (let i = 0; i < 16; i++) view.setUint16(at + 40 + i * 2, 0x0020, false);
	}
	image.set(identifier.subarray(0, 32), at + 40);

	both32(view, at + 80, plan.totalSectors);
	if (joliet) {
		// UCS-2 level 3 escape sequence, which is what marks this as Joliet.
		image.set(ascii('%/E'), at + 88);
	}
	both16(view, at + 120, 1);
	both16(view, at + 124, 1);
	both16(view, at + 128, SECTOR);
	both32(view, at + 132, joliet ? plan.jolietPathBytes : plan.primaryPathBytes);
	view.setUint32(at + 140, joliet ? plan.jolietPathL : plan.primaryPathL, true);
	view.setUint32(at + 144, 0, true);
	view.setUint32(at + 148, joliet ? plan.jolietPathM : plan.primaryPathM, false);
	view.setUint32(at + 152, 0, false);

	writeRecord(
		image,
		view,
		at + 156,
		Uint8Array.of(0),
		joliet ? root.jolietExtent : root.primaryExtent,
		joliet ? root.jolietSize : root.primarySize,
		true,
		date
	);

	image.fill(joliet ? 0x00 : 0x20, at + 190, at + 813);
	if (joliet) for (let i = 0; i < 311; i++) view.setUint16(at + 190 + i * 2, 0x0020, false);
	writeVolumeDate(image, at + 813, date);
	writeVolumeDate(image, at + 830, date);
	image.fill(0x30, at + 847, at + 863);
	image[at + 863] = 0;
	image.fill(0x30, at + 864, at + 880);
	image[at + 880] = 0;
	view.setUint8(at + 881, 1);
}

/**
 * @param root  tree of { name, isDir, size, children: Map, source }
 * @param readFile  async (node) => Uint8Array
 */
export async function buildIsoImage(
	root,
	{ readFile, onProgress, label = 'V86SANDBOX', date = new Date() } = {}
) {
	const plan = planIso(root);
	const image = new Uint8Array(plan.byteLength);
	const view = new DataView(image.buffer);

	writeDescriptor(image, view, 16, 1, plan, root, false, date, label);
	writeDescriptor(image, view, 17, 2, plan, root, true, date, label);
	// Volume descriptor set terminator.
	view.setUint8(18 * SECTOR, 0xff);
	image.set(ascii('CD001'), 18 * SECTOR + 1);
	view.setUint8(18 * SECTOR + 6, 1);

	writePathTable(image, view, plan.primaryPathL * SECTOR, plan.directories, false, true);
	writePathTable(image, view, plan.primaryPathM * SECTOR, plan.directories, false, false);
	writePathTable(image, view, plan.jolietPathL * SECTOR, plan.directories, true, true);
	writePathTable(image, view, plan.jolietPathM * SECTOR, plan.directories, true, false);

	const parentOf = new Map([[root, root]]);
	const mapParents = (node) => {
		for (const child of node.sorted) {
			if (!child.isDir) continue;
			parentOf.set(child, node);
			mapParents(child);
		}
	};
	mapParents(root);

	for (const { node } of plan.directories) {
		writeDirectoryExtent(image, view, node, parentOf.get(node), false, date);
		writeDirectoryExtent(image, view, node, parentOf.get(node), true, date);
	}

	let done = 0;
	for (const file of plan.files) {
		if (file.size === 0) continue;
		const bytes = await readFile(file);
		if (bytes.length !== file.size) {
			throw new Error(`"${file.name}" did not unpack correctly.`);
		}
		image.set(bytes, file.extent * SECTOR);
		done += file.size;
		onProgress?.({ done, name: file.name });
	}

	return { image, plan };
}
