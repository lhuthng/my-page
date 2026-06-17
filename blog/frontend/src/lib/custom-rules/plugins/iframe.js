export function iframeBlockPlugin(md) {
	md.block.ruler.before('fence', 'iframe', (state, startLine, endLine, silent) => {
		const pos = state.bMarks[startLine] + state.tShift[startLine];
		const max = state.eMarks[startLine];
		const line = state.src.slice(pos, max).trim();

		if (!line.startsWith(':::iframe')) return false;
		if (silent) return true;

		const parts = line.split(/\s+/);
		if (parts.length < 2) return false;

		const src = parts[1];
		const width = parts[2] || '100%';
		const height = parts[3] || '315';

		const token = state.push('iframe_block', '', 0);
		token.meta = { src, width, height };

		state.line = startLine + 1;
		return true;
	});

	md.renderer.rules.iframe_block = (tokens, idx) => {
		const { src, width, height } = tokens[idx].meta;

		if (!src.startsWith('http://') && !src.startsWith('https://')) {
			return `<p class="text-red-500">Invalid iframe source:</p>`;
		}

		const escapedSrc = String(src).replace(/"/g, '&quot;');

		const widthAttr = /^\d+$/.test(String(width)) ? `${width}px` : width;

		return `
      <div class="w-full rounded-lg custom-scrollbar overflow-hidden ">
        <iframe
          src="${escapedSrc}"
          width="${widthAttr}"
          height="${height}"
          frameborder="0"
          class="w-full"
          allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
          allowfullscreen
          loading="lazy">
        </iframe>
      </div>
    `;
	};
}
