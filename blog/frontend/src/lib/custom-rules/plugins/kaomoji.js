export function kaomojiPlugin(md) {
	md.inline.ruler.before('emphasis', 'kaomoji', (state, silent) => {
		const src = state.src;
		const start = state.pos;

		if (src[start] !== '@' || src[start + 1] !== '@' || src[start + 2] !== '[') {
			return false;
		}

		const closeIndex = src.indexOf(']@@', start + 3);
		if (closeIndex === -1) return false;

		if (silent) return false;

		const content = src.slice(start + 3, closeIndex);

		const token = state.push('kaomoji', '', 0);
		token.meta = { content };

		state.pos = closeIndex + 3;
		return true;
	});

	md.renderer.rules.kaomoji = (tokens, idx) => {
		const content = tokens[idx].meta.content;
		const escaped = md.utils.escapeHtml(content);
		return `<span class="kaomoji">${escaped}</span>`;
	};
}
