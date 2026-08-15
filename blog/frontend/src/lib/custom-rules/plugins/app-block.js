export function appBlockPlugin(md) {
	function normalizeCssSize(value, fallback) {
		if (!value || value === '_') return fallback;
		return /^\d+$/.test(value) ? `${value}px` : value;
	}

	md.block.ruler.before('fence', 'app', (state, startLine, endLine, silent) => {
		const pos = state.bMarks[startLine] + state.tShift[startLine];
		const max = state.eMarks[startLine];
		const line = state.src.slice(pos, max).trim();

		if (!line.startsWith(':::app')) return false;
		if (silent) return true;

		const parts = line.split(/\s+/);
		if (parts.length < 3) return false;

		const type = parts[1];
		const name = parts[2];
		const width = normalizeCssSize(parts[3], '100%');
		const height = normalizeCssSize(parts[4], '400px');
		const rest = parts.slice(5).join('-') || '';
		const mediaDictionary = state.env?.mediaDictionary || {};
		const temp = mediaDictionary[name];

		const token = state.push('app_block', '', 0);
		token.meta = { type, name, width, height, rest, temp };

		state.line = startLine + 1;
		return true;
	});

	md.renderer.rules.app_block = (tokens, idx) => {
		const { type, name, width, height, rest, temp } = tokens[idx].meta;

		let dataBlock =
			`
      data-name="${md.utils.escapeHtml(name)}" data-type="${md.utils.escapeHtml(type)}" data-width="${md.utils.escapeHtml(width)}" data-height="${md.utils.escapeHtml(height)}" ` +
			(rest ? `data-config="${md.utils.escapeHtml(rest)}" ` : '') +
			(temp ? `data-temp="${md.utils.escapeHtml(temp)}" ` : '');

		return `
      <div class="w-full">
        <div
          class="app-container mx-auto"
          ${dataBlock}
          style="width: ${md.utils.escapeHtml(width)}; min-height: ${md.utils.escapeHtml(height)};">
        </div>\n
      </div>
    `;
	};
}
