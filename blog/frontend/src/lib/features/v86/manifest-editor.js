// Two-way mapping between a v86 manifest string and the structured model the
// ManifestEditor component edits. The string stays the source of truth —
// parse on load, serialize on every edit — so save-time validation keeps
// working on the exact format the backend parses (INI-style key=value,
// comments ignored, variant root fallback).

/** INI subset the backend parses: `key=value` lines, `#`/`;` comments and `[sections]` skipped, last key wins, keys lower-cased. */
function parseFields(manifest) {
	const fields = new Map();
	for (const rawLine of String(manifest ?? '').split(/\r?\n/)) {
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

function keyIndex(base, key) {
	if (!key.startsWith(base)) return null;
	const rest = key.slice(base.length);
	if (rest.length === 0 || !/^\d+$/.test(rest)) return null;
	return Number.parseInt(rest, 10);
}

function resolveFor(fields, base, index, fallbackRoot) {
	if (index > 1) {
		return fields.get(`${base}${index}`) ?? (fallbackRoot ? fields.get(base) : undefined);
	}
	return fields.get(base) ?? fields.get(`${base}1`);
}

const SAVE_ALIASES = ['save_paths', 'save_path', 'saves'];
const KNOWN_BASES = ['name', 'exe', 'args'];
const GLOBAL_KEYS = new Set(['delay_ms', 'mouse_speed', 'revert_mouse_y', ...SAVE_ALIASES]);

function isTruthy(value) {
	return ['1', 'true', 'yes', 'on'].includes(
		String(value ?? '')
			.trim()
			.toLowerCase()
	);
}

/** Split a save-paths value the way the launcher does (`,` or `;` separated). */
export function splitSavePaths(raw) {
	return String(raw ?? '')
		.split(/[,;]/)
		.map((entry) => entry.trim().replace(/^"+|"+$/g, ''))
		.filter((entry) => entry.length > 0);
}

/**
 * @returns {{ variants: {name,exe,args}[], delayMs, mouseSpeed, revertMouseY,
 *   savePaths: string[], unknown: {key,value}[] }} — keys the format doesn't
 *   know are kept in `unknown` so serializing never silently drops data.
 */
export function parseManifestModel(manifest) {
	const fields = parseFields(manifest);

	const nameIndices = new Set();
	for (const key of fields.keys()) {
		if (key === 'name' || key === 'name1') {
			nameIndices.add(1);
		} else {
			const index = keyIndex('name', key);
			if (index !== null) nameIndices.add(Math.max(1, index));
		}
	}
	const count = nameIndices.size > 0 ? Math.max(...nameIndices) : 1;

	const variants = [];
	for (let i = 1; i <= count; i++) {
		variants.push({
			name: resolveFor(fields, 'name', i, false) ?? '',
			exe: resolveFor(fields, 'exe', i, true) ?? '',
			args: resolveFor(fields, 'args', i, true) ?? ''
		});
	}

	const rawSaves = SAVE_ALIASES.map((key) => fields.get(key)).find((v) => v !== undefined) ?? '';

	const unknown = [];
	for (const [key, value] of fields) {
		const base = KNOWN_BASES.find((b) => key === b || keyIndex(b, key) !== null);
		if (!base && !GLOBAL_KEYS.has(key)) unknown.push({ key, value });
	}

	return {
		variants,
		delayMs: fields.get('delay_ms') ?? '',
		mouseSpeed: fields.get('mouse_speed') ?? '',
		revertMouseY: isTruthy(fields.get('revert_mouse_y')),
		savePaths: splitSavePaths(rawSaves),
		unknown
	};
}

/**
 * Serialize back to manifest text. Variant 1 uses root keys; later variants
 * use suffixed keys only when set to something other than the root value, so
 * editing the root keeps flowing down to inheriting variants. Globals are
 * emitted only when non-default; unknown keys kept verbatim.
 */
export function serializeManifestModel(model) {
	const lines = [];
	const root = model.variants[0] ?? blankVariant();
	model.variants.forEach((variant, i) => {
		const index = i + 1;
		if (index === 1) {
			lines.push(`name=${variant.name}`);
			lines.push(`exe=${variant.exe}`);
			if (variant.args !== '') lines.push(`args=${variant.args}`);
			return;
		}
		if (variant.name !== '') lines.push(`name${index}=${variant.name}`);
		if (variant.exe !== '' && variant.exe !== root.exe) lines.push(`exe${index}=${variant.exe}`);
		if (variant.args !== '' && variant.args !== root.args)
			lines.push(`args${index}=${variant.args}`);
	});
	if (model.delayMs !== '') lines.push(`delay_ms=${model.delayMs}`);
	if (model.mouseSpeed !== '') lines.push(`mouse_speed=${model.mouseSpeed}`);
	if (model.revertMouseY) lines.push('revert_mouse_y=1');
	if (model.savePaths.length > 0) lines.push(`save_paths=${model.savePaths.join('; ')}`);
	for (const { key, value } of model.unknown ?? []) lines.push(`${key}=${value}`);
	return lines.join('\n');
}

export function blankVariant() {
	return { name: '', exe: '', args: '' };
}
