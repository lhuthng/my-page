// Rewrites content references the backend reports as moved, and marks the ones
// whose target is gone. The backend resolves links by slug at save time, so it
// is the only place that can tell a stale `/projects/old` apart from a valid
// one; this plugin just applies that answer to the rendered body.

const PATH_BY_KIND = {
	post: 'posts',
	project: 'projects',
	game: 'games',
	series: 'series'
};

const KIND_BY_PATH = {
	posts: 'post',
	projects: 'project',
	games: 'game',
	series: 'series'
};

const LABEL_BY_KIND = {
	post: 'This post',
	project: 'This project',
	game: 'This game',
	series: 'This series'
};

const HREF_RE = /(?:https?:\/\/[^\s)"'<>]+)?\/(posts|projects|games|series)\/([A-Za-z0-9_-]+)/;

const APP_EMBED_KIND = {
	project: 'project',
	html: 'project',
	v86: 'project',
	jsdos: 'project',
	game: 'game'
};

export function contentTargetFromHref(href) {
	const match = HREF_RE.exec(String(href ?? ''));
	if (!match) return null;
	return { entity_kind: KIND_BY_PATH[match[1]], slug: match[2] };
}

export function findLinkState(linkHealth, entityKind, slug) {
	if (!Array.isArray(linkHealth) || !slug) return null;
	// `content` covers posts, projects and games; their slugs are unique across
	// all three. Matching on it keeps a purged target matched even when the
	// report can no longer say which of the three it used to be.
	const targetKind = entityKind === 'series' ? 'series' : 'content';
	return (
		linkHealth.find(
			(link) => link.status !== 'ok' && link.target_kind === targetKind && link.target_slug === slug
		) ?? null
	);
}

/**
 * The app block names a project or game by slug. Returns the slug to embed and
 * whether the embed should render a missing state instead of mounting.
 */
export function resolveAppEmbed(linkHealth, type, name) {
	const entityKind = APP_EMBED_KIND[type];
	if (!entityKind) return { name, missing: false };
	const state = findLinkState(linkHealth, entityKind, name);
	if (!state) return { name, missing: false };
	if (state.status === 'missing') return { name, missing: true };
	return { name: state.current_slug ?? name, missing: false };
}

export function linkHealthPlugin(md) {
	const render =
		md.renderer.rules.link_open ??
		((tokens, idx, options, env, self) => self.renderToken(tokens, idx, options));

	md.renderer.rules.link_open = (tokens, idx, options, env, self) => {
		const token = tokens[idx];
		const target = contentTargetFromHref(token.attrGet('href'));
		const state = target && findLinkState(env?.linkHealth, target.entity_kind, target.slug);

		if (state?.status === 'moved' && state.current_slug) {
			token.attrSet('href', `/${PATH_BY_KIND[state.entity_kind]}/${state.current_slug}`);
			token.attrSet('data-link-moved', 'true');
		} else if (state?.status === 'missing') {
			// The href already names the kind, so it stays correct even if the
			// row was purged and the report lost that detail.
			token.attrSet('class', 'missing-link');
			token.attrSet(
				'title',
				`${LABEL_BY_KIND[target.entity_kind] ?? 'This link'} is no longer available.`
			);
			token.attrSet('data-link-missing', target.entity_kind);
		}

		return render(tokens, idx, options, env, self);
	};
}
