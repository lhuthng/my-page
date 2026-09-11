// Mirrors the backend's create-time field validation (helper::string in the
// Rust service) so cheap mistakes fail BEFORE a heavy upload starts, not
// after 200 MB have crossed the wire.
//
// Errors are keyed by field, not returned as a single string. A red line at
// the top of the page cannot say *which* input is wrong, so the editor renders
// these under the offending field instead. An empty object means valid.

const MAX_TITLE = 200;
const MAX_EXCERPT = 400;

function checkText(value, name, max) {
	const trimmed = (value ?? '').trim();
	if (!trimmed) return `${name} must not be empty.`;
	if ([...trimmed].length > max) return `${name} must be at most ${max} characters.`;
	return null;
}

function checkSlug(value) {
	const slug = (value ?? '').trim().toLowerCase();
	if (slug.length < 2 || slug.length > 100) return 'Slug must be between 2 and 100 characters.';
	if (!/^[a-z0-9_-]+$/.test(slug))
		return 'Slug may only contain lowercase letters, numbers, hyphens, and underscores.';
	return null;
}

/** Drop the nulls, so `{}` means "valid" and callers can test it directly. */
function collect(candidates) {
	const errors = {};
	for (const [field, message] of Object.entries(candidates)) {
		if (message) errors[field] = message;
	}
	return errors;
}

/** Per-field errors for the create flows, where all three fields are required. */
export function validateBasicsFields({ title, slug, excerpt } = {}) {
	return collect({
		title: checkText(title, 'Title', MAX_TITLE),
		slug: checkSlug(slug),
		excerpt: checkText(excerpt, 'Excerpt', MAX_EXCERPT)
	});
}

/** Per-field errors for whichever of title/slug/excerpt a PATCH would send. */
export function validatePatchFieldsMap(patch = {}) {
	return collect({
		title: patch.title !== undefined ? checkText(patch.title, 'Title', MAX_TITLE) : null,
		slug: patch.slug !== undefined ? checkSlug(patch.slug) : null,
		excerpt: patch.excerpt !== undefined ? checkText(patch.excerpt, 'Excerpt', MAX_EXCERPT) : null
	});
}
