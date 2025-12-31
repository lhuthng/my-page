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
    switch (src) {
      case undefined:
        return `<span class="missing-image">${value}</span>`;
      case null:
        return `<span class="loading-image">${value}</span>`;
      default: {
        const style = [];
        if (width && !isNaN(parseInt(width, 10)))
          style.push(`width:${width}px`);
        if (height && !isNaN(parseInt(height, 10)))
          style.push(`height:${height}px`);
        const styleAttr = style.length ? ` style="${style.join(";")}"` : "";

        switch (tag) {
          case "img":
            return `<img src="${src}" alt="${value}" ${styleAttr}/>`;
          case "img-inl":
            return `<img class="inline-block align-bottom" src="${src}" alt="${value}" ${styleAttr}/>`;
          case "img-left-float":
            const floatStyle = style.length
              ? `style="${style.join(";")}; float: left; margin-right: 5px; margin-bottom: 5px;"`
              : 'style="float: left; margin-right: 5px; margin-bottom: 5px;"';

            return `<img src="${src}" alt="${value}" ${floatStyle}/>`;
          default:
            return `<span class="invalid-tag">${tag}-${value}</span>`;
        }
      }
    }
  };
}

export function youtubeBlockPlugin(md) {
  md.block.ruler.before(
    "fence",
    "youtube",
    (state, startLine, endLine, silent) => {
      const pos = state.bMarks[startLine] + state.tShift[startLine];
      const max = state.eMarks[startLine];
      const line = state.src.slice(pos, max).trim();

      if (!line.startsWith(":::youtube")) return false;
      if (silent) return true;

      const parts = line.split(/\s+/);
      if (parts.length < 2) return false;

      const videoId = parts[1];
      const width = parts[2] || "560";
      const height = parts[3] || "315";

      const token = state.push("youtube_block", "", 0);
      token.meta = { videoId, width, height };

      state.line = startLine + 1;
      return true;
    },
  );

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
