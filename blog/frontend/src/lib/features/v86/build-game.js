import { unzip as fflateUnzip } from 'fflate';
import { buildFatDisk, sparseChunks, sparseChunkAt, DiskBuildError } from './fat-disk.js';
import { buildIsoImage } from '../../players/iso9660.js';
import { parseVariants, launcherConfigFor, ManifestError } from './manifest.js';
import { Sha256 } from './sha256.js';

export class GameBuildError extends Error {}

const ENCODER = new TextEncoder();
const HASH_CHUNK = 1024 * 1024;
/**
 * Fixed build date for content-addressed determinism: identical ZIPs/manifests
 * must produce identical disk and CD images so dedup by sha256 works. Directory
 * timestamps are cosmetic in Windows 9x, so a constant epoch is safe.
 */
export const DEFAULT_BUILD_DATE = new Date('2000-01-01T00:00:00Z');
const MAX_ZIP_BYTES = 500 * 1024 * 1024;
const MAX_EXTRACTED_BYTES = 1024 * 1024 * 1024;
const MAX_FILES = 10000;
const NESTED_SUFFIXES = ['.img', '.iso', '.jsdos', '.7z', '.rar'];

const RESERVED_WINDOWS = new Set(['CON', 'PRN', 'AUX', 'NUL']);

function isReservedWindowsName(component) {
	const stem = component.split('.')[0].replace(/ +$/g, '').toUpperCase();
	if (RESERVED_WINDOWS.has(stem)) return true;
	return (
		stem.length === 4 &&
		(stem.startsWith('COM') || stem.startsWith('LPT')) &&
		stem.charCodeAt(3) >= 0x31 &&
		stem.charCodeAt(3) <= 0x39
	);
}

/** Skips macOS-created junk that carries no game data (mirrors the server). */
function isMacosJunk(normalized) {
	const first = normalized.split('/')[0] ?? '';
	if (first.toUpperCase() === '__MACOSX') return true;
	const name = normalized.slice(normalized.lastIndexOf('/') + 1);
	return name.toUpperCase() === '.DS_STORE';
}

function isWindowsIncompatible(component) {
	if (isReservedWindowsName(component)) return true;
	if (component.endsWith(' ') || component.endsWith('.')) return true;
	for (const ch of component) {
		const code = ch.codePointAt(0);
		if (code < 0x20 || code === 0x7f) return true;
		if ('<>:"|?*'.includes(ch)) return true;
	}
	return false;
}

/**
 * Extracts (skipping macOS junk) and validates a game ZIP into
 * `[{ path, data }]`, mirroring the server's `validate_and_extract_game_zip`
 * plus `unwrap_single_top_level_dir`. Paths use `/` and land at the drive root.
 */
export async function unzipGame(zipBytes, limits = {}) {
	const maxZip = limits.maxZipBytes ?? MAX_ZIP_BYTES;
	const maxExtracted = limits.maxExtractedBytes ?? MAX_EXTRACTED_BYTES;
	const maxFiles = limits.maxFiles ?? MAX_FILES;
	if (zipBytes.byteLength > maxZip) {
		throw new GameBuildError('The game ZIP exceeds the configured limit.');
	}

	let decoded;
	try {
		decoded = await new Promise((resolve, reject) => {
			fflateUnzip(zipBytes, (error, result) => (error ? reject(error) : resolve(result)));
		});
	} catch (error) {
		throw new GameBuildError(
			`Invalid game ZIP: ${error?.message ?? error}${error?.code != null ? ` (code=${error.code})` : ''}`
		);
	}
	const raw = Object.entries(decoded);
	if (raw.length > maxFiles) {
		throw new GameBuildError(`The game ZIP exceeds the ${maxFiles} file limit.`);
	}

	const files = [];
	const seen = new Set();
	let expanded = 0;
	let extractedFiles = 0;
	for (const [entryName, data] of raw) {
		const normalized = entryName.replace(/\\/g, '/');
		if (normalized.includes('\0') || normalized.startsWith('/')) {
			throw new GameBuildError('The game ZIP contains an unsafe path.');
		}
		if (isMacosJunk(normalized)) continue;
		const components = normalized.split('/');
		if (components.includes('..')) {
			throw new GameBuildError('The game ZIP contains path traversal.');
		}
		for (const component of components) {
			if (isWindowsIncompatible(component)) {
				throw new GameBuildError('The game ZIP contains a Windows-incompatible path.');
			}
		}
		if (`D:\\${normalized}`.length >= 260) {
			throw new GameBuildError('The game ZIP contains a path longer than Windows 9x supports.');
		}
		const key = normalized.toLowerCase();
		if (seen.has(key)) {
			throw new GameBuildError('The game ZIP contains case-insensitive duplicate paths.');
		}
		seen.add(key);
		const lower = normalized.toLowerCase();
		if (NESTED_SUFFIXES.some((suffix) => lower.endsWith(suffix))) {
			throw new GameBuildError('Nested disk images and archives are not accepted.');
		}
		expanded += data.byteLength;
		if (expanded > maxExtracted) {
			throw new GameBuildError('The expanded game ZIP exceeds the configured limit.');
		}
		if (normalized.endsWith('/')) continue; // directory marker
		files.push({ path: normalized, data });
		extractedFiles += 1;
	}
	if (extractedFiles === 0) {
		throw new GameBuildError('The game ZIP contains no game files.');
	}

	return unwrapSingleTopLevelDir(files);
}

/**
 * Drops a single top-level wrapper directory (Game/Game/...) so the game lands
 * at the drive root, matching the server's `unwrap_single_top_level_dir`.
 */
function unwrapSingleTopLevelDir(files) {
	let current = files;
	while (true) {
		let top = null;
		let allUnderOne = true;
		for (const file of current) {
			const slash = file.path.indexOf('/');
			if (slash < 0) {
				allUnderOne = false;
				break;
			}
			const component = file.path.slice(0, slash);
			if (top === null) top = component;
			else if (component !== top) {
				allUnderOne = false;
				break;
			}
		}
		if (!allUnderOne || top === null) return current;
		current = current.map((file) => ({
			...file,
			path: file.path.slice(top.length + 1)
		}));
	}
}

/**
 * Builds the partitioned FAT game disk (D:) from `files` and returns the
 * sparse image, mirroring the server's `build_game_disk`.
 */
export function buildDisk(files, { now = DEFAULT_BUILD_DATE } = {}) {
	return buildFatDisk(files, { now });
}

/**
 * SHA-256 over the exact disk image bytes, streamed from the sparse layout in
 * small buffers so huge games never materialize in memory.
 */
export function diskSha(sparse) {
	const hasher = new Sha256();
	let offset = 0;
	for (const chunk of sparseChunks(sparse, HASH_CHUNK)) {
		const bytes = Math.min(HASH_CHUNK, sparse.size - offset);
		hasher.update(chunk.subarray(0, bytes));
		offset += HASH_CHUNK;
	}
	return hasher.digestHex();
}

/**
 * Hashes and zstd-compresses the disk in `chunkSize` chunks (last chunk
 * zero-padded to `chunkSize`, matching the server's `split_asset`). The disk
 * SHA-256 is over the exact image bytes so content-addressing matches the
 * server build. `zstdCompress` is injected (worker-backed in the browser); it
 * receives the raw chunk and returns its compressed bytes. The upload flow
 * splits at the server-returned `chunk_size_bytes`, not a fixed 8 MiB default.
 * Compression runs `workers` chunks concurrently; parts are stored by index so
 * the output stays ordered regardless of completion order.
 */
export async function buildDiskParts(
	sparse,
	chunkSize,
	zstdCompress,
	onChunk = () => {},
	{ workers = 4 } = {}
) {
	const sha256 = diskSha(sparse);
	const totalChunks = Math.ceil(sparse.size / chunkSize);
	const parts = new Array(totalChunks);
	let next = 0;
	let done = 0;
	await Promise.all(
		Array.from({ length: Math.min(workers, totalChunks) }, async () => {
			while (next < totalChunks) {
				const index = next++;
				const start = index * chunkSize;
				const rawChunk = sparseChunkAt(sparse, chunkSize, index);
				const bytes = Math.min(chunkSize, sparse.size - start);
				// Compress only the real bytes: the server verifies each part
				// decompresses to exactly `min(chunkSize, size - offset)` bytes.
				const compressed = await zstdCompress(rawChunk.subarray(0, bytes).slice().buffer);
				parts[index] = {
					offset: start,
					name: `${start}-${start + chunkSize}.img.zst`,
					rawBytes: bytes,
					compressed
				};
				onChunk(++done, totalChunks);
			}
		})
	);
	return {
		sha256,
		size_bytes: sparse.size,
		chunk_size_bytes: chunkSize,
		chunk_count: parts.length,
		parts
	};
}

/**
 * Builds one variant's launcher CD (E:): the autorun launcher, the in-guest
 * `[game]` config, the full manifest, and a small variant marker. Mirrors the
 * server's `build_game_cdrom`.
 */
export async function buildVariantIso({
	manifest,
	variant,
	launcherExe,
	now = DEFAULT_BUILD_DATE
}) {
	const root = { name: '', isDir: true, children: new Map() };
	const addFile = (name, content) => {
		const data = typeof content === 'string' ? ENCODER.encode(content) : content;
		root.children.set(name, { name, isDir: false, size: data.length, data });
	};
	addFile('AUTORUN.INF', '[autorun]\r\nopen=LAUNCHER.EXE\r\n');
	addFile('LAUNCHER.EXE', launcherExe);
	addFile('V86GAME.INI', launcherConfigFor(manifest, variant));
	addFile('V86GAME.MANIFEST', manifest);
	addFile('V86VARIANT.INI', `[variant]\r\nindex=${variant.index}\r\nname=${variant.name}\r\n`);
	const volumeId = variant.index <= 1 ? 'V86GAME' : `V86GAME${variant.index}`;
	const { image } = await buildIsoImage(root, {
		readFile: (node) => node.data,
		label: volumeId,
		date: now
	});
	const hasher = new Sha256();
	hasher.update(image);
	return {
		index: variant.index,
		name: variant.name,
		exe: variant.exe,
		args: variant.args,
		sha256: hasher.digestHex(),
		size_bytes: image.byteLength,
		bytes: image
	};
}

/**
 * Builds the launcher CDs for every manifest variant (used when only the
 * manifest changed and the stored disk is reused).
 */
export async function buildLauncherIsos({ manifest, launcherExe, now }) {
	const variants = parseVariants(manifest);
	const isos = [];
	for (const variant of variants) {
		isos.push(await buildVariantIso({ manifest, variant, launcherExe, now }));
	}
	return isos;
}

/**
 * Builds everything for a full upload: unzip -> game disk -> hash plus one
 * launcher CD per variant. Returns the raw disk (`sparse`) and its
 * content-addressing sha, plus the finished variant ISO bytes. The caller
 * splits/compresses the disk into parts AFTER the server's start response
 * returns the real `chunk_size_bytes`.
 */
export async function buildGame({
	zipBytes,
	manifest,
	launcherExe,
	now = DEFAULT_BUILD_DATE,
	onProgress = () => {},
	limits
}) {
	onProgress('Extracting game…');
	const files = await unzipGame(zipBytes, limits);
	onProgress('Building game disc…');
	const sparse = buildDisk(files, { now });
	const disk = { sparse, sha256: diskSha(sparse), size_bytes: sparse.size };
	onProgress('Building launcher…');
	const variants = await buildLauncherIsos({ manifest, launcherExe, now });
	return { files, disk, variants };
}

export function isGameBuildError(error) {
	return (
		error instanceof GameBuildError ||
		error instanceof DiskBuildError ||
		error instanceof ManifestError
	);
}
