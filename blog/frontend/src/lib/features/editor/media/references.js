/**
 * Inline media references — the single source of truth for how media is named
 * inside a post or project body.
 *
 * A body refers to media by short name while it is being edited
 * (`@[img:my-photo]`), but the backend rewrites those short names to numeric
 * indices before storing (`@[img:0]`), keyed by `post_media_usages.code`. That
 * indirection is deliberate: because the stored body holds an index rather than
 * a name, renaming a medium's short name does not have to rewrite every post
 * that references it.
 *
 * The decode half of that scheme previously existed in three separate copies
 * (the dashboard post loader, the dashboard project loader, and the public page
 * loaders), each with its own regex literals. They live here now.
 */

/** `@[img:key]`, optionally dimensioned as `@(200_100)[img:key]`. */
export const mediaSyntax = /@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;

/** `:::app glb-demo key` */
export const glbSyntax = /:::app\s+glb-demo\s+([^\s]+)/g;

/** `:::app lottie key` */
export const lottieAppSyntax = /:::app\s+lottie\s+([^\s]+)/g;

/**
 * Every pattern whose first capture group names a medium the editor must be
 * able to resolve. Broader than {@link DECODE_PATTERNS} because the editor also
 * checks glb references exist, even though they are not index-encoded.
 */
const COLLECT_PATTERNS = [mediaSyntax, glbSyntax, lottieAppSyntax];

/**
 * The patterns the backend actually rewrites to indices
 * (`extract_media_short_names` in `handlers/post.rs`). Decoding has to mirror
 * that set exactly: decoding a pattern the backend never encoded could rewrite
 * a key that was only coincidentally numeric.
 */
const DECODE_PATTERNS = [mediaSyntax, lottieAppSyntax];

/**
 * Collect the distinct media keys referenced by one or more bodies.
 *
 * @param {string | string[]} texts
 * @returns {string[]} unique keys, in first-seen order
 */
export function collectMediaKeys(texts) {
	const bodies = Array.isArray(texts) ? texts : [texts];
	const keys = new Set();

	for (const text of bodies) {
		if (!text) continue;
		for (const pattern of COLLECT_PATTERNS) {
			for (const match of text.matchAll(pattern)) {
				if (match[1]) keys.add(match[1]);
			}
		}
	}

	return [...keys];
}

/**
 * Replace the stored numeric indices in `text` with their short names, so the
 * editor shows `@[img:my-photo]` rather than `@[img:0]`.
 *
 * Edits are applied back-to-front: a replacement changes the length of the
 * string, so applying them in source order would invalidate every later index.
 *
 * An index with no corresponding short name is left exactly as it is rather
 * than being blanked — losing the reference would silently corrupt the body.
 *
 * @param {string} text
 * @param {string[]} shortNames indexed by the stored code
 * @returns {string}
 */
export function decodeShortNames(text, shortNames) {
	if (!text) return text ?? '';
	if (!shortNames?.length) return text;

	const edits = [];
	for (const pattern of DECODE_PATTERNS) {
		for (const match of text.matchAll(pattern)) {
			const key = match[1];
			if (key === undefined) continue;

			// Only a purely numeric key is a stored index; a body that already
			// holds short names must pass through untouched.
			if (!/^\d+$/.test(key)) continue;

			const replacement = shortNames[Number(key)];
			if (replacement === undefined || replacement === null) continue;

			edits.push({
				index: match.index + match[0].lastIndexOf(key),
				length: key.length,
				replacement
			});
		}
	}

	edits.sort((a, b) => b.index - a.index);

	let next = text;
	for (const { index, length, replacement } of edits) {
		next = next.slice(0, index) + replacement + next.slice(index + length);
	}
	return next;
}

/**
 * Build the render-time dictionary that maps a media key to its URL.
 *
 * Both forms are registered because a stored body refers to media by index
 * while an in-editor body refers to it by short name, and the same renderer
 * serves both.
 *
 * @param {string[]} mediumUrls indexed by stored code
 * @param {string[]} [mediumShortNames] parallel to `mediumUrls`
 * @param {(url: string) => string} [resolveUrl] e.g. `fixClientRoute`
 * @returns {Record<string, string>}
 */
export function buildMediaDictionary(mediumUrls, mediumShortNames, resolveUrl = (url) => url) {
	const dictionary = {};
	if (!mediumUrls?.length) return dictionary;

	mediumUrls.forEach((url, index) => {
		dictionary[index.toString()] = resolveUrl(url);
	});
	mediumShortNames?.forEach((shortName, index) => {
		if (shortName) dictionary[shortName] = resolveUrl(mediumUrls[index]);
	});

	return dictionary;
}
