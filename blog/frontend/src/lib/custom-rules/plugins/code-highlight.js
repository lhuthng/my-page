import hljs from 'highlight.js';

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
			.map((line) => `<span class="hljs-line">${line || ''}</span>`)
			.join('\n')}</code></pre>`;
	};
}
