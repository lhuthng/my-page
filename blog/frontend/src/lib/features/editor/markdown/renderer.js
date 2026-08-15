import MarkdownIt from 'markdown-it';
import mkKatex from 'markdown-it-katex';
import anchor from 'markdown-it-anchor';
// Relative rather than `$lib/...`, and from the individual plugin modules
// rather than `$lib/custom-rules/index.js`. Two reasons, both about being
// testable under a plain `node --test` run: node does not resolve the `$lib`
// alias, and that barrel re-exports `enhance.js`, which pulls in Svelte runes.
import { appBlockPlugin } from '../../../custom-rules/plugins/app-block.js';
import { codeHighlightPlugin } from '../../../custom-rules/plugins/code-highlight.js';
import { iframeBlockPlugin } from '../../../custom-rules/plugins/iframe.js';
import { kaomojiPlugin } from '../../../custom-rules/plugins/kaomoji.js';
import { mediaWithShortcutPlugin } from '../../../custom-rules/plugins/media-shortcut.js';
import { namedContainerPlugin } from '../../../custom-rules/plugins/named-container.js';
import { revealPlugin } from '../../../custom-rules/plugins/reveal.js';
import { youtubeBlockPlugin } from '../../../custom-rules/plugins/youtube.js';
import { slugify } from '../../../custom-rules/utils.js';

/**
 * The one markdown pipeline for post and project bodies.
 *
 * This exact plugin chain used to be spelled out in three places — the editor
 * preview, the public post page, and the public project page. Any plugin added
 * to one and not the others made the editor preview quietly disagree with what
 * readers would actually see. There is now a single definition, so a preview
 * that looks right *is* right.
 *
 * `html` is left at markdown-it's default of `false`: raw HTML in a body stays
 * escaped, and every rich construct goes through a custom plugin that escapes
 * its own interpolations.
 *
 * @returns {MarkdownIt} a fresh renderer
 */
export function createMarkdownRenderer() {
	return new MarkdownIt()
		.use(mkKatex)
		.use(mediaWithShortcutPlugin)
		.use(iframeBlockPlugin)
		.use(youtubeBlockPlugin)
		.use(appBlockPlugin)
		.use(revealPlugin)
		.use(namedContainerPlugin)
		.use(codeHighlightPlugin)
		.use(kaomojiPlugin)
		.use(anchor, { slugify });
}

/**
 * Render a body to HTML.
 *
 * @param {MarkdownIt} renderer
 * @param {string} body
 * @param {Record<string, string>} [mediaDictionary] key -> resolved URL
 * @returns {string}
 */
export function renderBody(renderer, body, mediaDictionary = {}) {
	return renderer.render(body ?? '', { mediaDictionary });
}
