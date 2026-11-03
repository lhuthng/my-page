import test from 'node:test';
import assert from 'node:assert/strict';
import { createCommentMarkdown, extractMentionUsernames } from '../syntax/markdown.js';
import { codeHighlightPlugin } from '../../../custom-rules/plugins/code-highlight.js';
import {
	codeHighlightLazyPlugin,
	loadHighlightJs
} from '../../../custom-rules/plugins/code-highlight-lazy.js';

const mentionDictionary = {
	alice: {
		username: 'alice',
		display_name: 'Alice A',
		avatar_url: 'media/avatars/alice'
	}
};

const content = [
	'hello @alice and @bob',
	'',
	'@@[ (｡•ᴗ•｡) ]@@',
	'',
	'```javascript',
	'const answer = 42;',
	'```'
].join('\n');

test('comment markdown renders mentions, kaomoji, and fenced code', async () => {
	await loadHighlightJs();
	const html = createCommentMarkdown({
		mentionDictionary,
		highlightPlugin: codeHighlightLazyPlugin
	}).render(content);

	assert.match(html, /class="mention-chip">@alice</);
	assert.match(html, /href="\/profiles\/alice"/);
	assert.match(html, / and @bob<|>@bob</);
	assert.doesNotMatch(html, /href="\/profiles\/bob"/);
	assert.match(html, /class="kaomoji"/);
	assert.match(html, /class="hljs-keyword">const</);
});

test('profile server rendering matches the client renderer', async () => {
	await loadHighlightJs();
	const shared = { mentionDictionary, highlightPlugin: codeHighlightLazyPlugin };
	const client = createCommentMarkdown(shared).render(content);
	const server = createCommentMarkdown({
		mentionDictionary,
		highlightPlugin: codeHighlightPlugin
	}).render(content);

	assert.equal(server, client);
});

test('extractMentionUsernames dedupes and skips email addresses', () => {
	assert.deepEqual(extractMentionUsernames('@alice hi @bob, @alice'), ['alice', 'bob']);
	assert.deepEqual(extractMentionUsernames('mail alice@example.com'), []);
	assert.deepEqual(extractMentionUsernames(''), []);
});
