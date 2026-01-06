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
          case "audio":
            return `<div class="audio-container"><audio src="${src}" alt="${value}" controls></audio></div>`;
          case "vid":
            return `<div class="video-container"><video ${styleAttr} alt="${value}" controls><source src="${src}"/></video></div>`;
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

export function appBlockPlugin(md) {
  md.block.ruler.before("fence", "app", (state, startLine, endLine, silent) => {
    const pos = state.bMarks[startLine] + state.tShift[startLine];
    const max = state.eMarks[startLine];
    const line = state.src.slice(pos, max).trim();

    if (!line.startsWith(":::app")) return false;
    if (silent) return true;

    const parts = line.split(/\s+/);
    if (parts.length < 3) return false;

    const type = parts[1];
    const name = parts[2];
    const width = parts[3] || "100%";
    const height = parts[4] || "400px";

    const token = state.push("app_block", "", 0);
    token.meta = { type, name, width, height };

    state.line = startLine + 1;
    return true;
  });

  md.renderer.rules.app_block = (tokens, idx) => {
    const { type, name, width, height } = tokens[idx].meta;

    // Return an empty div with data attributes
    return `
      <div class="w-full">
        <div
          class="app-container mx-auto"
          data-name="${name}"
          data-type="${type}"
          data-width="${width}px"
          data-height="${height}px"
          style="width: ${width}px; height: ${height}px;">
        </div>\n
      </div>
    `;
  };
}

export function revealPlugin(md) {
  function render(tokens, idx) {
    const token = tokens[idx];

    if (token.nesting === 1) {
      const title =
        token.info.trim().replace(/^reveal\s*/, "") || "Click to reveal";
      return (
        `<div class="reveal">` +
        `<button class="reveal-tooltip">${md.utils.escapeHtml(title)}</button>` +
        `<div class="reveal-content">`
      );
    }

    return "</div></div>";
  }

  md.block.ruler.before(
    "fence",
    "reveal",
    (state, startLine, endLine, silent) => {
      const startPos = state.bMarks[startLine] + state.tShift[startLine];
      const maxPos = state.eMarks[startLine];
      const line = state.src.slice(startPos, maxPos);

      if (!line.startsWith(":::< reveal")) return false;
      if (silent) return true;

      let nextLine = startLine + 1;

      // search for closing "::::"
      while (nextLine < endLine) {
        const pos = state.bMarks[nextLine] + state.tShift[nextLine];
        const max = state.eMarks[nextLine];
        const l = state.src.slice(pos, max);

        if (l.trim() === ":::>") break;
        nextLine++;
      }

      const tokenOpen = state.push("reveal_open", "div", 1);
      tokenOpen.block = true;
      tokenOpen.info = line.slice(4).trim(); // "reveal …"
      tokenOpen.map = [startLine, nextLine];

      state.md.block.tokenize(state, startLine + 1, nextLine);

      const tokenClose = state.push("reveal_close", "div", -1);
      tokenClose.block = true;

      state.line = nextLine + 1;
      return true;
    },
  );

  md.renderer.rules.reveal_open = render;
  md.renderer.rules.reveal_close = render;
}
