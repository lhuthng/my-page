export { pluginExtend } from './enhance.js';
export { slugify, findHeaders } from './utils.js';
export { mediaWithShortcutPlugin } from './plugins/media-shortcut.js';
export { youtubeBlockPlugin } from './plugins/youtube.js';
export { iframeBlockPlugin } from './plugins/iframe.js';
export { appBlockPlugin } from './plugins/app-block.js';
export { revealPlugin } from './plugins/reveal.js';
export { namedContainerPlugin } from './plugins/named-container.js';
// No re-export of code-highlight.js here on purpose: everything importing the
// barrel (e.g. Post.svelte) would statically pull in the full highlight.js.
// Import it from './plugins/code-highlight.js' directly, or use the lazy
// variant in './plugins/code-highlight-lazy.js' for client-side rendering.
export { mentionProfilePlugin } from './plugins/mention.js';
export { kaomojiPlugin } from './plugins/kaomoji.js';
