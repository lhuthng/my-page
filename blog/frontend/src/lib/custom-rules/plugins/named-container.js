export function namedContainerPlugin(md) {
	md.block.ruler.before('fence', 'named_container', (state, startLine, endLine, silent) => {
		const startPos = state.bMarks[startLine] + state.tShift[startLine];
		const maxPos = state.eMarks[startLine];
		const line = state.src.slice(startPos, maxPos).trim();

		if (!line.startsWith(':::container')) return false;

		const parts = line.trim().split(/\s+/);
		const name = parts[1];
		if (!name) return false;

		if (silent) return true;

		let nextLine = startLine + 1;
		while (nextLine < endLine) {
			const pos = state.bMarks[nextLine] + state.tShift[nextLine];
			const max = state.eMarks[nextLine];
			const l = state.src.slice(pos, max).trim();
			if (l === ':::') break;
			nextLine++;
		}

		const tokenOpen = state.push('named_container_open', 'div', 1);
		tokenOpen.block = true;
		tokenOpen.meta = { name };
		tokenOpen.map = [startLine, nextLine];

		state.md.block.tokenize(state, startLine + 1, nextLine);

		const tokenClose = state.push('named_container_close', 'div', -1);
		tokenClose.block = true;

		state.line = nextLine + 1;
		return true;
	});

	md.renderer.rules.named_container_open = (tokens, idx) => {
		const name = md.utils.escapeHtml(tokens[idx].meta.name);
		return `<div class="${name}-container">\n`;
	};

	md.renderer.rules.named_container_close = () => `</div>\n`;
}
