export function youtubeBlockPlugin(md) {
	md.block.ruler.before('fence', 'youtube', (state, startLine, endLine, silent) => {
		const pos = state.bMarks[startLine] + state.tShift[startLine];
		const max = state.eMarks[startLine];
		const line = state.src.slice(pos, max).trim();

		if (!line.startsWith(':::youtube')) return false;
		if (silent) return true;

		const parts = line.split(/\s+/);
		if (parts.length < 2) return false;

		const videoId = parts[1];
		const width = parts[2] || '560';
		const height = parts[3] || '315';

		const token = state.push('youtube_block', '', 0);
		token.meta = { videoId, width, height };

		state.line = startLine + 1;
		return true;
	});

	md.renderer.rules.youtube_block = (tokens, idx) => {
		const { videoId, width, height } = tokens[idx].meta;

		return `
      <div class="w-full rounded-lg overflow-hidden">
          <iframe
              width="100%"
              class="aspect-video"
              src="https://www.youtube.com/embed/${videoId}"
              frameborder="0"
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
              allowfullscreen>
          </iframe>
      </div>
    `;
	};
}
