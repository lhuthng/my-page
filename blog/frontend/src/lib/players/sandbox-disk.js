import { readZipDirectory, readZipEntry } from './zip.js';
import { buildIsoImage } from './iso9660.js';

// Mirrors the backend's extractor: macOS junk is dropped and a ZIP whose whole
// payload sits under one folder is unwrapped, so the game lands at the disc
// root rather than at D:\Game\.
const isMacJunk = (path) => {
	const [first] = path.split('/');
	if (first.toUpperCase() === '__MACOSX') return true;
	const name = path.slice(path.lastIndexOf('/') + 1);
	return name.toUpperCase() === '.DS_STORE' || name.startsWith('._');
};

const isReservedName = (name) => /^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(\.|$)/i.test(name);

const newDir = (name) => ({ name, isDir: true, size: 0, children: new Map() });

export function buildTree(entries) {
	const root = newDir('');
	let files = 0;
	let bytes = 0;

	for (const entry of entries) {
		const path = entry.name.replace(/\\/g, '/');
		if (path.endsWith('/') || isMacJunk(path)) continue;
		if (entry.encrypted) throw new Error(`"${path}" is password protected.`);

		const parts = path.split('/').filter((part) => part && part !== '.');
		if (parts.length === 0 || parts.some((part) => part === '..')) {
			throw new Error(`"${path}" has a bad path and was rejected.`);
		}
		if (parts.some(isReservedName)) {
			throw new Error(`"${path}" uses a name Windows will not allow.`);
		}

		let node = root;
		for (const part of parts.slice(0, -1)) {
			let next = node.children.get(part);
			if (!next) {
				next = newDir(part);
				node.children.set(part, next);
			} else if (!next.isDir) {
				throw new Error(`"${path}" clashes with another file.`);
			}
			node = next;
		}
		const name = parts[parts.length - 1];
		if (node.children.has(name)) throw new Error(`"${path}" is in the zip twice.`);
		node.children.set(name, {
			name,
			isDir: false,
			size: entry.size,
			children: new Map(),
			source: entry
		});
		files++;
		bytes += entry.size;
	}

	if (files === 0) throw new Error('That zip has no files in it.');
	return { root: unwrap(root), files, bytes };
}

function unwrap(root) {
	let node = root;
	while (node.children.size === 1) {
		const [only] = node.children.values();
		if (!only.isDir) break;
		node = only;
	}
	node.name = '';
	return node;
}

/**
 * Reads a game ZIP and returns a mountable CD image, entirely in the browser:
 * nothing is uploaded and nothing is compressed. A CD rather than a disk
 * because v86 can insert one into a machine that is already running.
 */
export async function buildSandboxIso(file, onProgress) {
	onProgress?.({ phase: 'reading', message: 'Opening your game…' });
	const entries = await readZipDirectory(file);
	const { root, files, bytes } = buildTree(entries);

	onProgress?.({
		phase: 'building',
		message: `Preparing ${files} files (${(bytes / 1048576).toFixed(1)} MB)…`
	});

	const { image, plan } = await buildIsoImage(root, {
		readFile: (node) => readZipEntry(file, node.source),
		onProgress: ({ done, name }) =>
			onProgress?.({
				phase: 'writing',
				message: `Adding ${name}`,
				percent: bytes > 0 ? Math.round((done / bytes) * 100) : 100
			})
	});

	return { image, plan, files, bytes };
}
