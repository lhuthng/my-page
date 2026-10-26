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
 * Render math through KaTeX, but never fall back to the raw source.
 *
 * `markdown-it-katex` returns the LaTeX source verbatim when KaTeX throws a
 * parse error. That string is not escaped, so it reaches `{@html}` as live
 * markup: `$<img src=x onerror=alert(1)>&$` is not valid LaTeX (the bare `&`),
 * and used to render as a real `<img>` tag. `html: false` does not cover it,
 * because that setting only escapes markdown-it's own HTML tokens, not a
 * plugin's return value.
 *
 * A successful KaTeX render is always an element, so any output that is byte
 * identical to the token source is the plugin's failure path. `markdown-it-katex`
 * registers both the tokenizer and the renderer; this wraps the renderers it
 * left behind, so it has to be mounted after it.
 *
 * @param {MarkdownIt} md
 */
function safeMathPlugin(md) {
	const renderInline = md.renderer.rules.math_inline;
	const renderBlock = md.renderer.rules.math_block;

	md.renderer.rules.math_inline = (tokens, idx, options, env, self) => {
		const rendered = renderInline(tokens, idx, options, env, self);
		const source = tokens[idx].content;
		if (rendered === source) {
			return `<code class="katex-error">${md.utils.escapeHtml(source)}</code>`;
		}
		return rendered;
	};

	md.renderer.rules.math_block = (tokens, idx, options, env, self) => {
		const rendered = renderBlock(tokens, idx, options, env, self);
		const source = tokens[idx].content;
		// The block renderer appends its own newline to whatever it is given.
		if (rendered === `${source}\n`) {
			return `<pre class="katex-error">${md.utils.escapeHtml(source)}</pre>\n`;
		}
		return rendered;
	};
}

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
 * its own interpolations. Math is the one exception worth spelling out: KaTeX
 * parses attacker-supplied source, so its error path is wrapped by
 * `safeMathPlugin` rather than trusted to return escaped output.
 *
 * @returns {MarkdownIt} a fresh renderer
 */
export function createMarkdownRenderer() {
	return new MarkdownIt()
		.use(mkKatex)
		.use(safeMathPlugin)
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
