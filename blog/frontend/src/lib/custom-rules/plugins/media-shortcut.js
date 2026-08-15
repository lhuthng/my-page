export function mediaWithShortcutPlugin(md) {
	md.inline.ruler.before('emphasis', 'extra', (state, silent) => {
		const start = state.pos;
		const src = state.src;

		if (src[start] !== '@') return false;

		let width, height;
		let keyStart;

		if (src[start + 1] === '(') {
			const closeParen = src.indexOf(')', start + 2);
			if (closeParen === -1) return false;

			const dim = src.slice(start + 2, closeParen).split('_');
			if (dim.length === 2) {
				width = dim[0].trim();
				height = dim[1].trim();
			}
			keyStart = closeParen + 1;
		} else {
			keyStart = start + 1;
		}

		if (src[keyStart] !== '[') return false;

		const closeBracket = src.indexOf(']', keyStart);
		if (closeBracket === -1) return false;

		let tag = undefined;
		let value = undefined;

		const firstColon = src.indexOf(':', keyStart);
		if (!(firstColon === -1 || firstColon > closeBracket)) {
			tag = src.slice(keyStart + 1, firstColon).trim();
			value = src.slice(firstColon + 1, closeBracket).trim();
		} else {
			value = src.slice(keyStart + 1, closeBracket).trim();
		}

		if (tag === '' || value === '') return false;

		if (silent) return false;

		const token = state.push('extra', '', 0);
		token.meta = { width, height, tag, value };

		state.pos = closeBracket + 1;
		return true;
	});

	const esc = (input) => md.utils.escapeHtml(String(input ?? ''));

	// Only accept a plain integer as a dimension. parseInt() alone would happily
	// accept `1"onload=...`, which then breaks out of the style attribute.
	const dimension = (input) => (/^\d+$/.test(String(input ?? '')) ? String(input) : null);

	md.renderer.rules.extra = (tokens, idx, options, env) => {
		const mediaDictionary = env?.mediaDictionary || {};
		const { width, height, tag, value } = tokens[idx].meta;
		const style = [];
		const w = dimension(width);
		const h = dimension(height);
		if (w) style.push(`width:${w}px`);
		if (h) style.push(`height:${h}px`);
		const styleAttr = style.length ? ` style="${style.join(';')}"` : '';
		const alt = esc(value);
		if (tag === undefined) {
			return `<img src="https://${esc(value)}" ${styleAttr}/>`;
		} else {
			const src = mediaDictionary[value];
			switch (src) {
				case undefined:
					return `<span class="missing-image">${alt}</span>`;
				case null:
					return `<span class="loading-image">${alt}</span>`;
				default: {
					const escapedSrc = esc(src);
					switch (tag) {
						case 'img':
							return `<img class="expandable" src="${escapedSrc}" alt="${alt}" ${styleAttr}/>`;
						case 'img-inl':
							return `<img class="expandable inline-block align-bottom" src="${escapedSrc}" alt="${alt}" ${styleAttr}/>`;
						case 'img-left-float':
							const floatStyle = style.length
								? `style="${style.join(';')}; float: left; margin-right: 5px; margin-bottom: 5px;"`
								: 'style="float: left; margin-right: 5px; margin-bottom: 5px;"';

							return `<img class="expandable" src="${escapedSrc}" alt="${alt}" ${floatStyle}/>`;
						case 'audio':
							return `<div class="audio-container"><audio src="${escapedSrc}" alt="${alt}" controls></audio></div>`;
						case 'vid':
							return `<div class="video-container"><video ${styleAttr} alt="${alt}" src="${escapedSrc}" controls></video></div>`;
						default:
							return `<span class="invalid-tag">${esc(tag)}-${alt}</span>`;
					}
				}
			}
		}
	};
}
