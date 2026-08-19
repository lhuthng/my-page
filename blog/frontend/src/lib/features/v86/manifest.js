const MANIFEST_MAX_BYTES = 64 * 1024;
const SAVE_FILE_MAX_LEN = 260;
const SAVE_FILE_MAX_COUNT = 64;

export class ManifestError extends Error {}

/** Parses a manifest into a lower-cased key -> value map (INI-style). */
function parseManifestFields(manifest) {
	const fields = new Map();
	for (const rawLine of manifest.split(/\r?\n/)) {
		const line = rawLine.trim();
		if (
			line.length === 0 ||
			line.startsWith('#') ||
			line.startsWith(';') ||
			(line.startsWith('[') && line.endsWith(']'))
		) {
			continue;
		}
		const eq = line.indexOf('=');
		if (eq >= 0) {
			fields.set(line.slice(0, eq).trim().toLowerCase(), line.slice(eq + 1).trim());
		}
	}
	return fields;
}

/** Lowest numeric suffix for a `{base}{digits}` key, or null. */
function keyIndex(base, key) {
	if (!key.startsWith(base)) return null;
	const rest = key.slice(base.length);
	if (rest.length === 0 || !/^\d+$/.test(rest)) return null;
	const index = Number.parseInt(rest, 10);
	return Number.isInteger(index) ? index : null;
}

/** Coalesces a key across a variant index with root fallback, matching the server. */
function resolveFor(fields, base, index, fallbackRoot) {
	if (index > 1) {
		return fields.get(`${base}${index}`) ?? (fallbackRoot ? fields.get(base) : undefined);
	}
	return fields.get(base) ?? fields.get(`${base}1`);
}

function normalizeManifestPath(value) {
	let normalized = value
		.trim()
		.replace(/^"+|"+$/g, '')
		.replace(/\\/g, '/');
	const upper = normalized.toUpperCase();
	if (upper.startsWith('D:/GAME/')) {
		normalized = normalized.slice(8);
	} else if (upper.startsWith('D:/')) {
		normalized = normalized.slice(3);
	}
	while (normalized.startsWith('./')) {
		normalized = normalized.slice(2);
	}
	if (normalized.length === 0 || normalized.startsWith('/')) {
		throw new ManifestError('The Windows 9x manifest contains an unsafe executable path.');
	}
	for (const component of normalized.split('/')) {
		if (component === '..' || component === '.' || component === '') {
			throw new ManifestError('The Windows 9x manifest contains an unsafe executable path.');
		}
	}
	return normalized;
}

/** Validates a single save entry from the manifest. */
function validateSaveFile(entry) {
	if (entry.length === 0) throw new ManifestError('Invalid Windows 9x save entry: empty.');
	if (entry.length > SAVE_FILE_MAX_LEN) {
		throw new ManifestError(`Invalid Windows 9x save entry '${entry}': too long.`);
	}
	for (const component of entry.split(/[/\\]/)) {
		if (component.length === 0) {
			throw new ManifestError(`Invalid Windows 9x save entry '${entry}': empty path component.`);
		}
		if (component === '..' || component === '.') {
			throw new ManifestError(`Invalid Windows 9x save entry '${entry}': unsafe path component.`);
		}
		for (const ch of component) {
			const code = ch.codePointAt(0);
			if (code < 0x21 || code > 0x7e) {
				throw new ManifestError(`Invalid Windows 9x save entry '${entry}': unsupported character.`);
			}
			if (';=:"<>|?*'.includes(ch)) {
				throw new ManifestError(`Invalid Windows 9x save entry '${entry}': unsupported character.`);
			}
		}
	}
}

/**
 * Resolves the manifest's launch variants. Names are the source of truth:
 * variant count comes from the highest named index, names must be contiguous
 * from 1..K, and every variant must resolve an executable. Projects with no
 * `name` keys inherit a single (unnamed) variant.
 */
export function parseVariants(manifest) {
	if (manifest.length > MANIFEST_MAX_BYTES) {
		throw new ManifestError('The v86 manifest cannot exceed 64 KiB.');
	}
	if (manifest.includes('\0')) {
		throw new ManifestError('The v86 manifest cannot contain NUL characters.');
	}
	const fields = parseManifestFields(manifest);

	const nameIndices = new Set();
	for (const key of fields.keys()) {
		if (key === 'name' || key === 'name1') {
			nameIndices.add(1);
		} else {
			const index = keyIndex('name', key);
			if (index !== null) nameIndices.add(Math.max(1, index));
		}
	}
	const maxName = nameIndices.size > 0 ? Math.max(...nameIndices) : undefined;

	let explicitMax = 0;
	for (const key of fields.keys()) {
		for (const base of ['name', 'exe', 'args']) {
			const index = keyIndex(base, key);
			if (index !== null) explicitMax = Math.max(explicitMax, index);
		}
	}

	let k;
	if (maxName !== undefined) {
		k = maxName;
	} else {
		if (explicitMax > 1) {
			throw new ManifestError('Variant keys (nameN/exeN/argsN) require a name for variant 1.');
		}
		k = 1;
	}

	for (let i = 1; i <= k; i++) {
		const named = i === 1 ? fields.has('name') || fields.has('name1') : fields.has(`name${i}`);
		if (!named) {
			throw new ManifestError(
				`The v86 manifest must name each variant contiguously (missing name for variant ${i}).`
			);
		}
	}
	if (explicitMax > k) {
		throw new ManifestError(
			`Variant keys reference index ${explicitMax} but only ${k} named variants exist.`
		);
	}

	const variants = [];
	for (let i = 1; i <= k; i++) {
		const name = resolveFor(fields, 'name', i, false) ?? '';
		let exe = resolveFor(fields, 'exe', i, true);
		if (!exe) {
			throw new ManifestError(`Variant ${i} requires an executable (exe${i} or exe).`);
		}
		exe = exe.trim();
		if (exe.length === 0) {
			throw new ManifestError(`Variant ${i} requires an executable (exe${i} or exe).`);
		}
		exe = normalizeManifestPath(exe);
		if (!exe.toLowerCase().endsWith('.exe')) {
			throw new ManifestError(
				`The Windows 9x manifest executable for variant ${i} must be an .exe file.`
			);
		}
		const args = resolveFor(fields, 'args', i, true) ?? '';
		variants.push({ index: i, name, exe, args });
	}
	return variants;
}

/** Resolves the manifest's save entries (e.g. `Save0001.dat; A/save0001.dat`). */
export function saveFilesFromManifest(manifest) {
	const fields = parseManifestFields(manifest);
	const raw = fields.get('save_paths') ?? fields.get('save_path') ?? fields.get('saves') ?? '';
	const files = [];
	const seen = new Set();
	for (let entry of raw.split(/[,;]/)) {
		entry = entry.trim().replace(/^"+|"+$/g, '');
		if (entry.length === 0) continue;
		validateSaveFile(entry);
		const normalized = entry.replace(/\//g, '\\');
		const key = normalized.toLowerCase();
		if (!seen.has(key)) {
			seen.add(key);
			files.push(normalized);
			if (files.length >= SAVE_FILE_MAX_COUNT) break;
		}
	}
	return files;
}

/**
 * Builds the in-guest launcher `[game]` config for a single variant, byte for
 * byte what the server's `launcher_config_for` produced.
 */
export function launcherConfigFor(manifest, variant) {
	const fields = parseManifestFields(manifest);
	const relativeWindows = variant.exe.replace(/\//g, '\\');
	const executable = `D:\\${relativeWindows}`;
	if (executable.length >= 260) {
		throw new ManifestError('The resolved Windows 9x executable path is too long.');
	}
	const slash = variant.exe.lastIndexOf('/');
	const workingDirectory =
		slash > 0 ? `D:\\${variant.exe.slice(0, slash).replace(/\//g, '\\')}` : 'D:\\';
	const arguments_ = variant.args;
	const delayMs = fields.get('delay_ms') ?? '1000';
	if (!/^\d+$/.test(delayMs)) {
		throw new ManifestError('The Windows 9x manifest delay_ms must be a number.');
	}

	const saveFiles = saveFilesFromManifest(manifest);
	let config = `[game]\r\nexecutable=${executable}\r\nworking_directory=${workingDirectory}\r\narguments=${arguments_}\r\ndelay_ms=${delayMs}\r\n`;
	if (saveFiles.length > 0) {
		config += '[saves]\r\n';
		for (const file of saveFiles) {
			config += `file=${file}\r\n`;
		}
	}
	return config;
}
