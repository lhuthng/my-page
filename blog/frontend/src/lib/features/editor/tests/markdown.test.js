import test from 'node:test';
import assert from 'node:assert/strict';
import { createMarkdownRenderer, renderBody } from '../markdown/renderer.js';
import { buildMediaDictionary, decodeShortNames } from '../media/references.js';

const FIXTURE = [
	'# Heading one',
	'',
	'Some **bold** text with an image: @[img:photo]',
	'',
	':::app lottie spinner',
	'',
	':::container note',
	'inside the container',
	':::',
	'',
	'```js',
	'const x = 1;',
	'```',
	'',
	'| a | b |',
	'| - | - |',
	'| 1 | 2 |'
].join('\n');

const DICTIONARY = { photo: '/media/i/photo.png', spinner: '/media/i/spinner.lottie' };

test('the editor preview and the published page render identically', () => {
	// The whole point of a single renderer: two independently-created instances
	// (one per call site) must produce byte-identical output.
	const editorRenderer = createMarkdownRenderer();
	const serverRenderer = createMarkdownRenderer();

	assert.equal(
		renderBody(editorRenderer, FIXTURE, DICTIONARY),
		renderBody(serverRenderer, FIXTURE, DICTIONARY)
	);
});

test('a renderer instance is reusable across calls', () => {
	// The server pages keep one module-scoped instance and render every request
	// through it, so rendering must not leave state behind.
	const renderer = createMarkdownRenderer();
	const first = renderBody(renderer, FIXTURE, DICTIONARY);
	renderBody(renderer, '# something else entirely', {});
	const third = renderBody(renderer, FIXTURE, DICTIONARY);

	assert.equal(first, third);
});

test('the full plugin chain is wired up', () => {
	const renderer = createMarkdownRenderer();
	const html = renderBody(renderer, FIXTURE, DICTIONARY);

	assert.match(html, /<h1 id="heading-one"/, 'anchor plugin should add heading ids');
	assert.match(html, /<img[^>]+src="\/media\/i\/photo\.png"/, 'media shortcut should resolve');
	assert.match(html, /class="app-container/, 'app block should render a mount point');
	assert.match(html, /class="note-container"/, 'named container should render');
	assert.match(html, /class="hljs"/, 'code highlighting should apply');
	assert.match(html, /<table>/, 'standard markdown should still work');
});

test('raw HTML in a body stays escaped', () => {
	const renderer = createMarkdownRenderer();
	const html = renderBody(renderer, '<script>alert(1)</script>', {});

	assert.doesNotMatch(html, /<script>/);
	assert.match(html, /&lt;script&gt;/);
});

test('renders a stored body once its indices are decoded', () => {
	// The end-to-end shape the loaders use: decode indices, build the
	// dictionary, then render.
	const shortNames = ['photo'];
	const urls = ['/media/i/photo.png'];
	const stored = 'text @[img:0] more';

	const html = renderBody(
		createMarkdownRenderer(),
		decodeShortNames(stored, shortNames),
		buildMediaDictionary(urls, shortNames)
	);

	assert.match(html, /<img[^>]+src="\/media\/i\/photo\.png"/);
});

test('an unresolved media key renders a marker instead of a broken image', () => {
	const html = renderBody(createMarkdownRenderer(), '@[img:nope]', {});
	assert.match(html, /class="missing-image"/);
});

test('rendering tolerates an empty or missing body', () => {
	const renderer = createMarkdownRenderer();
	assert.equal(renderBody(renderer, ''), '');
	assert.equal(renderBody(renderer, undefined), '');
	assert.equal(renderBody(renderer, null), '');
});
