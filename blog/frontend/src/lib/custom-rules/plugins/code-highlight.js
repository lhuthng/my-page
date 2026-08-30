import hljs from 'highlight.js';
import { highlightBlock } from './highlight-block.js';

// Static import on purpose: this variant serves the server-side body renderer
// and the editor preview, which both need synchronous highlighting.
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

		return highlightBlock(highlighted);
	};
}
