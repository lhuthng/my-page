import { highlightBlock } from './highlight-block.js';

// Comment code blocks highlight on demand: the comment engine never touches
// this module's imports until a code block actually renders, so article pages
// stop shipping the full highlight.js language table eagerly. lib/core plus a
// curated language set keeps the lazy chunk small; anything unregistered
// degrades to escaped text, exactly like an unknown language does today.
let hljs = null;
let hljsPromise = null;

const LANGUAGE_NAMES = [
	'bash',
	'c',
	'cpp',
	'csharp',
	'css',
	'diff',
	'go',
	'java',
	'javascript',
	'json',
	'kotlin',
	'markdown',
	'php',
	'plaintext',
	'python',
	'ruby',
	'rust',
	'sql',
	'typescript',
	'xml',
	'yaml'
];

// One literal import per language: a variable import path would make the
// bundler glob every language in the package into this chunk.
const LANGUAGE_IMPORTS = [
	import('highlight.js/lib/core'),
	import('highlight.js/lib/languages/bash'),
	import('highlight.js/lib/languages/c'),
	import('highlight.js/lib/languages/cpp'),
	import('highlight.js/lib/languages/csharp'),
	import('highlight.js/lib/languages/css'),
	import('highlight.js/lib/languages/diff'),
	import('highlight.js/lib/languages/go'),
	import('highlight.js/lib/languages/java'),
	import('highlight.js/lib/languages/javascript'),
	import('highlight.js/lib/languages/json'),
	import('highlight.js/lib/languages/kotlin'),
	import('highlight.js/lib/languages/markdown'),
	import('highlight.js/lib/languages/php'),
	import('highlight.js/lib/languages/plaintext'),
	import('highlight.js/lib/languages/python'),
	import('highlight.js/lib/languages/ruby'),
	import('highlight.js/lib/languages/rust'),
	import('highlight.js/lib/languages/sql'),
	import('highlight.js/lib/languages/typescript'),
	import('highlight.js/lib/languages/xml'),
	import('highlight.js/lib/languages/yaml')
];

export function loadHighlightJs() {
	hljsPromise ??= Promise.all(LANGUAGE_IMPORTS).then(([core, ...languageModules]) => {
		hljs = core.default;
		languageModules.forEach((language, i) => {
			hljs.registerLanguage(LANGUAGE_NAMES[i], language.default);
		});
		return hljs;
	});
	return hljsPromise;
}

export function codeHighlightLazyPlugin(md) {
	md.options.highlight = function (code, lang) {
		let highlighted;

		code = code.trimEnd();

		if (!hljs && !hljsPromise) {
			loadHighlightJs();
		}

		if (hljs && lang && hljs.getLanguage(lang)) {
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
