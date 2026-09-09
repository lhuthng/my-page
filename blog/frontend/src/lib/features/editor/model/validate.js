// Mirrors the backend's create-time field validation (helper::string in the
// Rust service) so cheap mistakes fail BEFORE a heavy upload starts, not
// after 200 MB have crossed the wire.

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

/** All of title/slug/excerpt are required — create flows. */
export function validateBasics({ title, slug, excerpt } = {}) {
	return (
		checkText(title, 'Title', MAX_TITLE) ??
		checkSlug(slug) ??
		checkText(excerpt, 'Excerpt', MAX_EXCERPT)
	);
}

/** Validate whichever of title/slug/excerpt a PATCH would send — edit flows. */
export function validatePatchFields(patch = {}) {
	return (
		(patch.title !== undefined ? checkText(patch.title, 'Title', MAX_TITLE) : null) ??
		(patch.slug !== undefined ? checkSlug(patch.slug) : null) ??
		(patch.excerpt !== undefined ? checkText(patch.excerpt, 'Excerpt', MAX_EXCERPT) : null)
	);
}
