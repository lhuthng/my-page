// Relative rather than `$lib/...` so this module stays loadable by a plain
// `node --test` run, which does not resolve SvelteKit's alias.
import { arraysEqualIgnoreOrder } from '../../../utils/index.js';

/**
 * Split the editor's space-joined tag textarea into a deduplicated-by-nothing,
 * trimmed tag list — the shape the backend expects. (Deduplication happens
 * server-side; see `resolve_tag_ids` in the backend.)
 *
 * @param {string} tagsText
 * @returns {string[]}
 */
export function splitTags(tagsText) {
	return (tagsText ?? '')
		.trim()
		.split(/\s+/)
		.filter((tag) => tag !== '');
}

/**
 * Trim, drop blanks, and keep only complete label+url pairs — mirrors the
 * backend's own `normalize_links`.
 *
 * @param {{label: string, url: string}[]} links
 */
export function normalizeLinks(links) {
	return (links ?? [])
		.map((link) => ({ label: (link.label ?? '').trim(), url: (link.url ?? '').trim() }))
		.filter((link) => link.label && link.url);
}

/**
 * Build the project-specific half of a PATCH payload.
 *
 * @param {{baseline: object, current: object}} args
 */
function buildProjectPatch({ baseline, current }) {
	const patch = {};

	const baseDemoType = baseline.demoType ?? 'html5';
	if (current.demoType !== baseDemoType) patch.demo_type = current.demoType;

	const baseWidth = baseline.demoWidth ?? '100%';
	if (current.demoWidth !== baseWidth) patch.demo_width = current.demoWidth;

	const baseHeight = baseline.demoHeight ?? '520px';
	if (current.demoHeight !== baseHeight) patch.demo_height = current.demoHeight;

	const baseUrl = baseline.demoUrl ?? '';
	if (current.demoType === 'none') {
		if (baseUrl !== '') patch.demo_url = '';
	} else if (
		!['none', 'html5', 'webgl', 'game'].includes(current.demoType) &&
		current.demoUrl !== baseUrl
	) {
		patch.demo_url = current.demoUrl;
	}

	const baseDelegate = baseline.delegateGameId ?? '';
	if ((current.delegateGameId ?? '') !== baseDelegate) {
		patch.delegate_game_id = current.delegateGameId ? Number(current.delegateGameId) : null;
	}
	if ((current.inheritThumbnail ?? true) !== (baseline.inheritThumbnail ?? true)) {
		patch.inherit_thumbnail = current.inheritThumbnail ?? true;
	}
	if ((current.inheritTags ?? true) !== (baseline.inheritTags ?? true)) {
		patch.inherit_tags = current.inheritTags ?? true;
	}

	// Both sides go through the same normalization: `baseline.links` can be the
	// unnormalized `[{label:'GitHub',url:''}]` placeholder `loadEntryState`
	// substitutes for an empty link list, which would otherwise compare unequal
	// to the normalized (and therefore empty) form of that exact same
	// placeholder on `current` — reporting a diff where the user changed
	// nothing.
	const links = normalizeLinks(current.links);
	if (JSON.stringify(links) !== JSON.stringify(normalizeLinks(baseline.links))) {
		patch.links = links;
	}

	return patch;
}

/**
 * Build the game-specific half of a PATCH payload. Games have no external
 * links; instead they carry the "many bodies" (instruction/cheatcode/story)
 * and related-game links.
 *
 * @param {{baseline: object, current: object}} args
 */
function buildGamePatch({ baseline, current }) {
	const patch = {};

	const baseDemoType = baseline.demoType ?? 'html5';
	if (current.demoType !== baseDemoType) patch.launcher_type = current.demoType;

	const baseWidth = baseline.demoWidth ?? '100%';
	if (current.demoWidth !== baseWidth) patch.demo_width = current.demoWidth;

	const baseHeight = baseline.demoHeight ?? '520px';
	if (current.demoHeight !== baseHeight) patch.demo_height = current.demoHeight;

	// jsdos/v86/html5/webgl never carry a URL.
	const baseUrl = baseline.demoUrl ?? '';
	if (
		!['html5', 'webgl', 'jsdos', 'v86'].includes(current.demoType) &&
		current.demoUrl !== baseUrl
	) {
		patch.demo_url = current.demoUrl;
	}

	if ((current.instruction ?? '') !== (baseline.instruction ?? '')) {
		patch.instruction = current.instruction ?? '';
	}
	if ((current.cheatcode ?? '') !== (baseline.cheatcode ?? '')) {
		patch.cheatcode = current.cheatcode ?? '';
	}
	if ((current.story ?? '') !== (baseline.story ?? '')) {
		patch.story = current.story ?? '';
	}

	const related = (current.relatedGames ?? [])
		.filter((link) => link.id !== '' && link.id != null)
		.map((link) => ({ id: Number(link.id), title: link.title ?? '', slug: link.slug ?? '' }));
	const baseRelated = (baseline.relatedGames ?? []).map((link) => ({
		id: Number(link.id),
		title: link.title ?? '',
		slug: link.slug ?? ''
	}));
	if (JSON.stringify(related) !== JSON.stringify(baseRelated)) {
		patch.related_games = related;
	}

	return patch;
}

/**
 * Build the partial PATCH payload for saving a post, project, or game: only
 * fields that differ from `baseline` — the state the server last confirmed —
 * are included.
 *
 * This is the single place that decides "what changed." It replaces the
 * hand-written if-chains that used to live inline in `PostEditor.svelte` and
 * `ProjectEditor.svelte`, each diffing against a `data` prop that was never
 * refreshed after a save — so every save after the first re-sent fields that
 * hadn't actually changed since the *previous* save. Called against a
 * `baseline` that the caller refreshes after each successful save, a second
 * save with no further edits produces an empty patch.
 *
 * @param {object} args
 * @param {object} args.baseline last-confirmed server state
 * @param {object} args.current current editor state (same shape as baseline)
 * @param {boolean} [args.hasNewMedia] force content+draft even if the draft
 *   text itself is unchanged — newly uploaded inline media still needs its
 *   server-side usage rows, which the backend derives from the body text.
 * @param {'post'|'project'|'game'} [args.kind]
 * @returns {Record<string, unknown>}
 */
export function buildPatch({ baseline, current, hasNewMedia = false, kind = 'post' }) {
	const patch = {};

	if (current.title !== baseline.title) patch.title = current.title;
	if (current.slug !== baseline.slug) patch.slug = current.slug;
	if (current.excerpt !== baseline.excerpt) patch.excerpt = current.excerpt;

	const tags = splitTags(current.tags);
	if (!arraysEqualIgnoreOrder(tags, baseline.tags ?? [])) patch.tags = tags;

	const contentChanged = current.bodies.draft !== baseline.bodies.draft;
	if (contentChanged || hasNewMedia) {
		patch.draft = current.bodies.draft;
		patch.content = current.bodies.content;
	}

	if ((current.ogImageSeconds ?? 0) !== (baseline.ogImageSeconds ?? 0)) {
		patch.og_image_seconds = current.ogImageSeconds ?? 0;
	}

	if (kind === 'project') {
		Object.assign(patch, buildProjectPatch({ baseline, current }));
	} else if (kind === 'game') {
		Object.assign(patch, buildGamePatch({ baseline, current }));
	}

	return patch;
}

/** @param {Record<string, unknown>} patch */
export function isPatchEmpty(patch) {
	return Object.keys(patch).length === 0;
}
