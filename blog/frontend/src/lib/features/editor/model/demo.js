/**
 * The project demo-type state machine: validation, v86 manifest parsing, and
 * the side effects of switching demo type — lifted as-is from
 * `ProjectEditor.svelte` so it can be tested and reused independent of the
 * component.
 */

export const DEMO_TYPES = [
	{ value: 'none', label: 'No Demo', disabled: false },
	{ value: 'html5', label: 'HTML5', disabled: false },
	{ value: 'embed', label: 'Embed', disabled: false },
	{ value: 'webgl', label: 'WebGL', disabled: false },
	{ value: 'jsdos', label: 'js-dos', disabled: false },
	{ value: 'v86', label: 'v86', disabled: false },
	{ value: 'download', label: 'Download', disabled: false },
	{ value: 'video', label: 'Video', disabled: false }
];

/**
 * What changes when the demo type is switched to `nextType`.
 *
 * This used to be a `$effect` reacting to `editingData.demoType`, which meant
 * it also ran once on mount in edit mode — silently wiping `demoUrl` for an
 * existing html5/webgl/jsdos/v86 project the instant the editor opened. As a
 * plain function it only runs when a caller explicitly invokes it from a
 * "change the demo type" action, never as a side effect of loading data.
 *
 * @param {string} nextType
 * @returns {{demoType: string, demoUrl?: string, demoZip?: undefined, demoZipName?: string, demoZipError?: string}}
 */
export function applyDemoTypeTransition(nextType) {
	const patch = { demoType: nextType };

	if (nextType === 'embed' || nextType === 'download' || nextType === 'video') {
		patch.demoZip = undefined;
		patch.demoZipName = '';
		patch.demoZipError = '';
	} else if (nextType === 'none') {
		patch.demoZip = undefined;
		patch.demoZipName = '';
		patch.demoZipError = '';
		patch.demoUrl = '';
	} else {
		// html5 / webgl / jsdos / v86: these never carry a URL.
		patch.demoUrl = '';
	}

	return patch;
}

/**
 * Parse a v86 manifest's `key = value` lines into a flat, lower-cased-key
 * object. Blank lines, `#`/`;` comments, and `[section]` headers are ignored.
 *
 * @param {string} manifest
 * @returns {Record<string, string>}
 */
export function parseV86Fields(manifest) {
	const fields = {};
	for (const rawLine of (manifest ?? '').split('\n')) {
		const line = rawLine.trim();
		if (!line || line.startsWith('#') || line.startsWith(';')) continue;
		if (line.startsWith('[') && line.endsWith(']')) continue;
		const eq = line.indexOf('=');
		if (eq === -1) continue;
		const key = line.slice(0, eq).trim().toLowerCase();
		const value = line.slice(eq + 1).trim();
		if (key) fields[key] = value;
	}
	return fields;
}

/**
 * Validate a v86 manifest's launch-variant keys (`name`/`nameN`, `exe`/`exeN`,
 * `args`/`argsN`). Variant indices must be contiguous starting at 1, each
 * variant needs a name and an `.exe`.
 *
 * @param {string} manifest
 * @returns {string | null} an error message, or `null` if valid
 */
export function v86VariantError(manifest) {
	const fields = parseV86Fields(manifest);
	const nameIndices = new Set();
	for (const key of Object.keys(fields)) {
		const m = key.match(/^name(\d+)?$/);
		if (m) nameIndices.add(m[1] ? Number(m[1]) : 1);
	}
	const maxName = nameIndices.size ? Math.max(...nameIndices) : 0;
	let explicitMax = 0;
	for (const key of Object.keys(fields)) {
		const m = key.match(/^(name|exe|args)(\d+)?$/);
		if (m) explicitMax = Math.max(explicitMax, m[2] ? Number(m[2]) : 1);
	}
	const k = maxName || (explicitMax > 1 ? 0 : 1);
	if (maxName === 0 && explicitMax > 1) {
		return 'Variant keys (nameN/exeN/argsN) require a name for variant 1.';
	}
	for (let i = 1; i <= k; i++) {
		const named = i === 1 ? 'name' in fields || 'name1' in fields : `name${i}` in fields;
		if (!named) return `Missing name for variant ${i} (names must be contiguous).`;
		const exe = fields[`exe${i}`] ?? fields.exe;
		if (!exe) return `Variant ${i} requires an executable (exe${i} or exe).`;
		if (!exe.trim().toLowerCase().endsWith('.exe')) {
			return `Variant ${i} executable must be an .exe file.`;
		}
	}
	if (explicitMax > k) {
		return `Variant keys reference index ${explicitMax} but only ${k} named variant(s) exist.`;
	}
	return null;
}

/**
 * Validate the demo-type-specific requirements before create/update submits.
 *
 * @param {object} args
 * @param {string} args.demoType
 * @param {string} args.demoUrl
 * @param {File | undefined} args.zip the pending demo zip, if any
 * @param {'create'|'edit'} args.mode
 * @param {string} [args.previousDemoType] `data?.demoType` in edit mode
 * @param {string} [args.v86SystemVersionId]
 * @param {string} [args.v86Manifest]
 * @returns {{valid: true} | {valid: false, error: string}}
 */
export function validateDemoFields({
	demoType,
	demoUrl,
	zip,
	mode,
	previousDemoType,
	v86SystemVersionId,
	v86Manifest
}) {
	switch (demoType) {
		case 'none':
			break;
		case 'html5':
		case 'webgl':
			if (mode === 'create' && !zip) {
				return {
					valid: false,
					error: `Zip file is required for ${demoType.toUpperCase()} projects.`
				};
			}
			break;
		case 'jsdos':
			if (mode === 'create' && !zip) {
				return { valid: false, error: 'A .jsdos bundle is required for js-dos projects.' };
			}
			break;
		case 'v86': {
			if (!v86SystemVersionId) return { valid: false, error: 'Select a v86 system.' };
			if ((mode === 'create' || previousDemoType !== 'v86') && !zip) {
				return { valid: false, error: 'A game ZIP is required for v86 projects.' };
			}
			if (new TextEncoder().encode(v86Manifest ?? '').length > 65536) {
				return { valid: false, error: 'The v86 manifest cannot exceed 64 KiB.' };
			}
			const variantIssue = v86VariantError(v86Manifest);
			if (variantIssue) return { valid: false, error: variantIssue };
			break;
		}
		case 'embed':
			if (!demoUrl) return { valid: false, error: 'Demo URL is required for Embed projects.' };
			break;
		case 'download':
			if (!demoUrl) return { valid: false, error: 'Download URL is required.' };
			break;
		case 'video':
			if (!demoUrl) return { valid: false, error: 'Video URL is required.' };
			break;
		default:
			return { valid: false, error: `Unsupported demo type: ${demoType}` };
	}
	return { valid: true };
}
