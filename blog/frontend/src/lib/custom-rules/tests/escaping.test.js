import test from 'node:test';
import assert from 'node:assert/strict';
import MarkdownIt from 'markdown-it';
import { appBlockPlugin } from '../plugins/app-block.js';
import { iframeBlockPlugin } from '../plugins/iframe.js';
import { mediaWithShortcutPlugin } from '../plugins/media-shortcut.js';
import { youtubeBlockPlugin } from '../plugins/youtube.js';
import { namedContainerPlugin } from '../plugins/named-container.js';
import { revealPlugin } from '../plugins/reveal.js';

const md = new MarkdownIt()
	.use(mediaWithShortcutPlugin)
	.use(iframeBlockPlugin)
	.use(youtubeBlockPlugin)
	.use(appBlockPlugin)
	.use(namedContainerPlugin)
	.use(revealPlugin);

const render = (src) => md.render(src, { mediaDictionary: { pic: 'https://cdn.example/pic.png' } });

// An unescaped `"` immediately before an `on*=` handler means the value broke
// out of its attribute. The escaped form (`&quot;onload=`) is inert text.
const escapesAttribute = (html) => /"\s*on[a-z]+\s*=/i.test(html);

test('block directives do not allow attribute breakout', () => {
	const attacks = [
		[':::iframe https://example.com 100"onload=alert(1) 200', 'iframe width'],
		[':::iframe https://example.com 100 200"onload=alert(1)', 'iframe height'],
		[':::youtube abc"onload=alert(1)', 'youtube id'],
		[':::app lottie name 100"onload=alert(1) 300', 'app style width'],
		[':::app lottie name 100px 300"onload=alert(1)', 'app style height'],
		[':::container foo"onload=alert(1)\ntext\n:::', 'container class name'],
		[':::< reveal "onload=alert(1)\nhidden\n:::>', 'reveal title']
	];

	for (const [src, label] of attacks) {
		assert.equal(escapesAttribute(render(src)), false, `${label} broke out of its attribute`);
	}
});

test('media shortcut escapes interpolated values', () => {
	// Unknown key: the key is echoed as text content and must be escaped.
	assert.match(
		render('@[img:<img src=x onerror=alert(1)>]'),
		/&lt;img src=x onerror=alert\(1\)&gt;/,
		'missing-key label was not escaped'
	);

	// Known key still renders a normal img.
	const ok = render('@[img:pic]');
	assert.match(ok, /<img class="expandable" src="https:\/\/cdn\.example\/pic\.png"/);
	assert.equal(escapesAttribute(ok), false);
});

test('media shortcut only accepts integer dimensions', () => {
	assert.match(render('@(200_100)[img:pic]'), /style="width:200px;height:100px"/);
	// A non-integer dimension is dropped rather than interpolated.
	const out = render('@(1x_2y)[img:pic]');
	assert.doesNotMatch(out, /style=/);
	assert.equal(escapesAttribute(out), false);
});

test('youtube rejects ids outside its charset', () => {
	assert.match(render(':::youtube dQw4w9WgXcQ'), /youtube\.com\/embed\/dQw4w9WgXcQ/);
	assert.match(render(':::youtube not/an/id'), /Invalid YouTube video id/);
});

test('iframe still requires an http(s) source', () => {
	assert.match(render(':::iframe javascript:alert(1)'), /Invalid iframe source/);
	assert.match(render(':::iframe https://example.com'), /src="https:\/\/example\.com"/);
});
