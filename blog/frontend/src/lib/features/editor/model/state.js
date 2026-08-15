// Relative rather than `$lib/...` so this module stays loadable by a plain
// `node --test` run.
import { nowToDate } from '../../../utils/index.js';

/**
 * The blank entry state for a brand-new post or project.
 *
 * @param {'post'|'project'} kind
 * @param {object[]} [initialSeries] series list to seed the post picker with
 * @returns {object}
 */
export function createEntryState(kind, initialSeries = []) {
	const base = {
		id: '',
		title: '',
		slug: '',
		excerpt: '',
		tags: '',
		bodies: { draft: '', content: '' },
		coverUrl: '',
		videoShortName: '',
		ogImageSeconds: 0,
		coverMediaType: '',
		date: nowToDate()
	};

	if (kind === 'post') {
		return {
			...base,
			series: initialSeries,
			seriesSlug: '',
			pendingSeriesId: null,
			relatedPosts: []
		};
	}

	return {
		...base,
		postId: '',
		demoType: 'html5',
		demoWidth: '100%',
		demoHeight: '520px',
		demoUrl: '',
		v86SystemVersionId: '',
		v86Manifest: '',
		v86ArtifactRevision: 0,
		links: [{ label: 'GitHub', url: '' }]
	};
}

/**
 * Map a loaded post/project into the editor's `{ entry, baseline }` pair in
 * one pass, so the two can never drift into different shapes of the same
 * source data.
 *
 * `entry` is what the form binds to: `tags` as the textarea's space-joined
 * string, plus a fresh `date` and (for posts) a `pendingSeriesId` slot for an
 * in-progress series pick.
 *
 * `baseline` is the diff target `buildPatch` compares against: `tags` stays
 * an array (as the server sends it), and it carries `updatedAt` — the
 * optimistic-lock token. A successful save replaces `baseline` wholesale with
 * the server's response, which is what keeps `buildPatch` from re-sending
 * fields on every subsequent save.
 *
 * @param {object} data the loaded post/project, in the shape `+page.svelte`
 *   already unpacks it into (see `PostEditor.svelte` / `ProjectEditor.svelte`)
 * @param {'post'|'project'} kind
 * @returns {{entry: object, baseline: object}}
 */
export function loadEntryState(data, kind) {
	const tags = data.tags ?? [];
	const bodies = { draft: data.draft, content: data.content };
	const common = {
		id: data.id,
		title: data.title,
		slug: data.slug,
		excerpt: data.excerpt,
		coverUrl: data.coverUrl,
		videoShortName: data.videoShortName ?? '',
		ogImageSeconds: data.ogImageSeconds ?? 0,
		coverMediaType: data.cover_media_type ?? data.coverMediaType
	};

	const extra =
		kind === 'post'
			? {
					series: data.series ?? [],
					seriesSlug: data.seriesSlug ?? '',
					relatedPosts: data.relatedPosts ?? []
				}
			: {
					postId: data.postId ?? '',
					demoType: data.demoType ?? 'html5',
					demoWidth: data.demoWidth ?? '100%',
					demoHeight: data.demoHeight ?? '520px',
					demoUrl: data.rawDemoUrl ?? '',
					v86SystemVersionId: data.v86SystemVersionId?.toString() ?? '',
					v86Manifest: data.v86Manifest ?? '',
					v86ArtifactRevision: data.v86ArtifactRevision ?? 0,
					links: data.links?.length ? data.links : [{ label: 'GitHub', url: '' }]
				};

	const baseline = {
		...common,
		...extra,
		tags,
		bodies: { ...bodies },
		updatedAt: data.updatedAt ?? data.updated_at ?? null
	};

	const entry = {
		...common,
		...extra,
		tags: tags.join(' '),
		bodies: { ...bodies },
		date: nowToDate(),
		...(kind === 'post' ? { pendingSeriesId: null } : {})
	};

	return { entry, baseline };
}

/**
 * Fold a successful save's response back into the baseline, so the next
 * `buildPatch` diffs against what the server now actually has.
 *
 * @param {object} baseline the current baseline
 * @param {object} entry the entry state as it was submitted (already saved)
 * @param {{updatedAt?: string}} [serverResponse] fields the server echoed back
 */
export function refreshBaseline(baseline, entry, serverResponse = {}) {
	return {
		...baseline,
		title: entry.title,
		slug: entry.slug,
		excerpt: entry.excerpt,
		tags: entry.tags
			.trim()
			.split(/\s+/)
			.filter((t) => t !== ''),
		bodies: { ...entry.bodies },
		ogImageSeconds: entry.ogImageSeconds,
		...(entry.demoType !== undefined
			? {
					demoType: entry.demoType,
					demoWidth: entry.demoWidth,
					demoHeight: entry.demoHeight,
					demoUrl: entry.demoUrl,
					links: entry.links
				}
			: {}),
		updatedAt: serverResponse.updatedAt ?? baseline.updatedAt
	};
}
