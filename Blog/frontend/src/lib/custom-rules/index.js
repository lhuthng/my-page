export function mediaWithShortcutPlugin(md, options) {
	const mediaDictionary = options.mediaDictionary || {};

	md.inline.ruler.before("emphasis", "extra", (state, silent) => {
		const start = state.pos;
		const src = state.src;

		if (src[start] !== "@") return false;

		let width, height;
		let keyStart;

		if (src[start + 1] === "(") {
			const closeParen = src.indexOf(")", start + 2);
			if (closeParen === -1) return false;

			const dim = src.slice(start + 2, closeParen).split("_");
			if (dim.length === 2) {
				width = dim[0].trim();
				height = dim[1].trim();
			}
			keyStart = closeParen + 1;
		} else {
			keyStart = start + 1;
		}

		if (src[keyStart] !== "[") return false;

		const closeBracket = src.indexOf("]", keyStart);
		if (closeBracket === -1) return false;

		const firstColon = src.indexOf(":", keyStart);
		if (firstColon === -1 || firstColon > closeBracket) return false;

		const tag = src.slice(keyStart + 1, firstColon).trim();
		const value = src.slice(firstColon + 1, closeBracket).trim();

		if (!tag || !value) return false;

		if (silent) return false;

		const token = state.push("extra", "", 0);
		token.meta = { width, height, tag, value };

		state.pos = closeBracket + 1;
		return true;
	});

	// Renderer
	md.renderer.rules.extra = (tokens, idx) => {
		const { width, height, tag, value } = tokens[idx].meta;
		const src = mediaDictionary[value];
		if (!src) return `<span class="missing-image">${value}</span>`;

		const style = [];
		if (width && !isNaN(parseInt(width, 10))) style.push(`width:${width}`);
		if (height && !isNaN(parseInt(height, 10))) style.push(`height:${height}`);
		const styleAttr = style.length ? ` style="${style.join(";")}"` : "";

		if (tag === "img") {
			return `<img src="${src}" alt="${value}"${styleAttr}/>`;
		} else {
			return `<span class="invalid-tag">${tag}-${value}</span>`;
		}
	};
}
