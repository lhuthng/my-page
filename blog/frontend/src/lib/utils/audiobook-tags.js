/**
 * Helpers for the audiobook tag vocabulary.
 *
 * Tag slugs come from the backend's ASCII-only slugify, so the slug for
 * "Vietnamese Translated" is stable and safe to match against.
 */

/** Marks an audiobook as a Vietnamese translation of an original work. */
export const VIETNAMESE_TRANSLATED_TAG = 'vietnamese-translated';

/**
 * Whether an audiobook carries the Vietnamese-translation tag.
 *
 * Accepts both tag shapes the API returns: listings expose parallel `tags`
 * (names) and `tag_slugs` string arrays, while the detail endpoint returns
 * `tags` as `{ name, slug }` objects. Prefer `tag_slugs` when present —
 * matching on names would break if a tag is renamed without changing slug.
 */
export function isVietnameseTranslation(audiobook) {
	const slugs =
		audiobook?.tag_slugs ??
		(audiobook?.tags ?? []).map((tag) => (typeof tag === 'string' ? tag : tag?.slug));

	return (slugs ?? []).includes(VIETNAMESE_TRANSLATED_TAG);
}
