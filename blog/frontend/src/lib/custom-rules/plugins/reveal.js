export function revealPlugin(md) {
	function render(tokens, idx) {
		const token = tokens[idx];

		if (token.nesting === 1) {
			const title = token.info.trim().replace(/^reveal\s*/, '') || 'Click to reveal';
			return (
				`<div class="reveal">` +
				`<button class="reveal-tooltip">${md.utils.escapeHtml(title)}</button>` +
				`<div class="reveal-content">`
			);
		}

		return '</div></div>';
	}

	md.block.ruler.before('fence', 'reveal', (state, startLine, endLine, silent) => {
		const startPos = state.bMarks[startLine] + state.tShift[startLine];
		const maxPos = state.eMarks[startLine];
		const line = state.src.slice(startPos, maxPos);

		if (!line.startsWith(':::< reveal')) return false;
		if (silent) return true;

		let nextLine = startLine + 1;

		while (nextLine < endLine) {
			const pos = state.bMarks[nextLine] + state.tShift[nextLine];
			const max = state.eMarks[nextLine];
			const l = state.src.slice(pos, max);

			if (l.trim() === ':::>') break;
			nextLine++;
		}

		const tokenOpen = state.push('reveal_open', 'div', 1);
		tokenOpen.block = true;
		tokenOpen.info = line.slice(4).trim();
		tokenOpen.map = [startLine, nextLine];

		state.md.block.tokenize(state, startLine + 1, nextLine);

		const tokenClose = state.push('reveal_close', 'div', -1);
		tokenClose.block = true;

		state.line = nextLine + 1;
		return true;
	});

	md.renderer.rules.reveal_open = render;
	md.renderer.rules.reveal_close = render;
}
