/** Shared <pre> template for the static and lazy code highlight plugins. */
export function highlightBlock(highlighted) {
	return `<pre class="hljs"><code>${highlighted
		.split(/\n/)
		.map((line) => `<span class="hljs-line">${line || ''}</span>`)
		.join('\n')}</code></pre>`;
}
