export function mentionProfilePlugin(md, options = {}) {
	const mentionDictionary = options.mentionDictionary || {};

	const normalizeAvatar = (url) => {
		if (!url) return '/anonymous.gif';
		if (url.startsWith('http://') || url.startsWith('https://')) return url;
		if (url.startsWith('/api/') || url.startsWith('/')) return url;
		return `/api/${String(url).replace(/^\.?\//, '')}`;
	};

	md.inline.ruler.before('emphasis', 'mention_profile', (state, silent) => {
		const src = state.src;
		const start = state.pos;

		if (src[start] !== '@') return false;

		const prev = start > 0 ? src[start - 1] : '';
		if (prev && /[A-Za-z0-9_.-]/.test(prev)) return false;

		let end = start + 1;
		while (end < src.length && /[A-Za-z0-9_-]/.test(src[end])) {
			end++;
		}

		const username = src.slice(start + 1, end);
		if (username.length < 3) return false;

		if (silent) return false;

		const token = state.push('mention_profile', '', 0);
		token.meta = { username };
		state.pos = end;
		return true;
	});

	md.renderer.rules.mention_profile = (tokens, idx) => {
		const username = tokens[idx].meta.username;
		const profile = mentionDictionary[username];

		if (!profile) {
			return md.utils.escapeHtml(`@${username}`);
		}

		const displayName = md.utils.escapeHtml(profile.display_name || username);
		const safeUsername = md.utils.escapeHtml(username);
		const safeAvatar = md.utils.escapeHtml(normalizeAvatar(profile.avatar_url));

		return `
      <a href="/profiles/${safeUsername}" class="mention-link">
        <span class="mention-chip">@${safeUsername}</span>
        <span class="mention-preview" aria-hidden="true">
          <img class="mention-preview-avatar" src="${safeAvatar}" alt="${displayName} avatar" />
          <span class="mention-preview-name">${displayName}</span>
        </span>
      </a>
    `;
	};
}
