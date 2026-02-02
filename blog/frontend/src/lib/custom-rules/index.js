import App from "$lib/components/App.svelte";
import { mount } from "svelte";
import hljs from "highlight.js";

export function pluginExtend(root) {
  const appContainers = root.querySelectorAll(".app-container");
  appContainers.forEach((container) => {
    if (container.__mounted) return;
    container.__mounted = true;

    const { name, type, width, height, config } = container.dataset;

    mount(App, {
      target: container,
      props: { name, type, width, height, config },
    });
  });

  const revealContainers = root.querySelectorAll(".reveal");
  revealContainers.forEach((container) => {
    if (container.__mounted) return;
    container.__mounted = true;

    const button = container.querySelector(".reveal-tooltip");

    button.addEventListener("click", () => {
      container.classList.toggle("toggled");
    });
  });

  const audioSyncContainers = root.querySelectorAll(".audio-sync-container");
  audioSyncContainers.forEach((container) => {
    if (container.__mounted) return;
    container.__mounted = true;

    const audios = container.querySelectorAll(".audio-container audio");
    let isSyncing = false;

    const syncPlay = () => {
      audios.forEach((audio) => {
        audio.play();
      });
    };

    const syncPause = () => {
      if (isSyncing) return;
      audios.forEach((audio) => {
        audio.pause();
      });
    };

    const syncTime = (time) => {
      audios.forEach((audio) => {
        audio.currentTime = time;
      });
    };

    audios.forEach((audio) => {
      audio.addEventListener("play", syncPlay);
      audio.addEventListener("pause", syncPause);
    });

    const duoBtn = document.createElement("div");
    duoBtn.className = "mx-auto w-fit duo-btn duo-dark";
    const btn = document.createElement("button");
    btn.textContent = "Sync Time";
    duoBtn.append(btn);
    container.appendChild(duoBtn);
    btn.addEventListener("click", () => {
      let avg = 0;
      audios.forEach((audio) => (avg += audio.currentTime / audios.length));
      syncTime(avg);
    });
  });
}

export function slugify(text) {
  return text
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9\s-]/g, "")
    .trim()
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-");
}

export function findHeaders(root) {
  const headers = root.querySelectorAll("h1, h2, h3");
  const next = { H1: "H2", H2: "H3", H3: "H4" };
  const result = [];
  let queue = [{ tagName: "H1", target: result }];
  headers.forEach(({ id, tagName, textContent }) => {
    let count = 0;
    const max = 100;

    const peek = queue.at(-1);
    if (tagName == peek.tagName) {
      peek.target.push({ id, textContent });
    } else if (tagName > peek.tagName) {
      let nextName = peek.tagName;
      count = 0;
      do {
        count++;
        if (count > max) return;
        nextName = next[nextName];
        const children = [];
        queue.at(-1).target.push(children);
        queue.push({ tagName: nextName, target: children });
      } while (tagName !== nextName);
      queue.at(-1).target.push({ id, textContent });
    } else {
      while (tagName !== queue.at(-1).tagName) {
        queue.pop();
      }
      queue.at(-1).target.push({ id, textContent });
    }
  });
  return result;
}

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

    let tag = undefined;
    let value = undefined;

    const firstColon = src.indexOf(":", keyStart);
    if (!(firstColon === -1 || firstColon > closeBracket)) {
      tag = src.slice(keyStart + 1, firstColon).trim();
      value = src.slice(firstColon + 1, closeBracket).trim();
    } else {
      value = src.slice(keyStart + 1, closeBracket).trim();
    }

    if (tag === "" || value === "") return false;

    if (silent) return false;

    const token = state.push("extra", "", 0);
    token.meta = { width, height, tag, value };

    state.pos = closeBracket + 1;
    return true;
  });

  // Renderer
  md.renderer.rules.extra = (tokens, idx) => {
    const { width, height, tag, value } = tokens[idx].meta;
    const style = [];
    if (width && !isNaN(parseInt(width, 10))) style.push(`width:${width}px`);
    if (height && !isNaN(parseInt(height, 10)))
      style.push(`height:${height}px`);
    const styleAttr = style.length ? ` style="${style.join(";")}"` : "";
    if (tag === undefined) {
      return `<img src="https://${value}" ${styleAttr}/>`;
    } else {
      const src = mediaDictionary[value];
      switch (src) {
        case undefined:
          return `<span class="missing-image">${value}</span>`;
        case null:
          return `<span class="loading-image">${value}</span>`;
        default: {
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
              return `<div class="video-container"><video ${styleAttr} alt="${value}" src="${src}" controls></video></div>`;
            default:
              return `<span class="invalid-tag">${tag}-${value}</span>`;
          }
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
    const width = !parts[3] || parts[3] === "_" ? "100%" : parts[3];
    const height = !parts[4] || parts[4] === "_" ? "400px" : parts[4];
    const rest = parts.slice(5).join("-") || "";

    const token = state.push("app_block", "", 0);
    token.meta = { type, name, width, height, rest };

    state.line = startLine + 1;
    return true;
  });

  md.renderer.rules.app_block = (tokens, idx) => {
    const { type, name, width, height, rest } = tokens[idx].meta;

    const dataBlock =
      `
      data-name="${name}"
      data-type="${type}"
      data-width="${width}px"
      data-height="${height}px"
    ` + (rest ? `data-config=${rest}` : "");

    // Return an empty div with data attributes
    return `
      <div class="w-full">
        <div
          class="app-container mx-auto"
          ${dataBlock}
          style="width: ${width}px; min-height: ${height}px;">
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

export function namedContainerPlugin(md) {
  md.block.ruler.before(
    "fence",
    "named_container",
    (state, startLine, endLine, silent) => {
      const startPos = state.bMarks[startLine] + state.tShift[startLine];
      const maxPos = state.eMarks[startLine];
      const line = state.src.slice(startPos, maxPos).trim();

      if (!line.startsWith(":::container")) return false;

      const parts = line.trim().split(/\s+/);
      const name = parts[1];
      if (!name) return false; // must have a name

      if (silent) return true;

      let nextLine = startLine + 1;
      while (nextLine < endLine) {
        const pos = state.bMarks[nextLine] + state.tShift[nextLine];
        const max = state.eMarks[nextLine];
        const l = state.src.slice(pos, max).trim();
        if (l === ":::") break;
        nextLine++;
      }

      // Open token
      const tokenOpen = state.push("named_container_open", "div", 1);
      tokenOpen.block = true;
      tokenOpen.meta = { name };
      tokenOpen.map = [startLine, nextLine];

      // Tokenize inner content
      state.md.block.tokenize(state, startLine + 1, nextLine);

      // Close token
      const tokenClose = state.push("named_container_close", "div", -1);
      tokenClose.block = true;

      state.line = nextLine + 1;
      return true;
    },
  );

  md.renderer.rules.named_container_open = (tokens, idx) => {
    const name = md.utils.escapeHtml(tokens[idx].meta.name);
    return `<div class="${name}-container">\n`;
  };

  md.renderer.rules.named_container_close = () => `</div>\n`;
}

export function codeHighlightPlugin(md) {
  md.options.highlight = function (code, lang) {
    let highlighted;

    code = code.trimEnd();

    if (lang && hljs.getLanguage(lang)) {
      try {
        highlighted = hljs.highlight(code, { language: lang }).value;
      } catch {}
    }

    if (!highlighted) {
      highlighted = md.utils.escapeHtml(code);
    }

    return `<pre class="hljs"><code>${highlighted
      .split(/\n/)
      .map((line) => `<span class="hljs-line">${line || ""}</span>`)
      .join("\n")}</code></pre>`;
  };
}
