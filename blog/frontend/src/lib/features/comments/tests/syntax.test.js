import test from 'node:test';
import assert from 'node:assert/strict';
import { MentionHandler, SlashCommandHandler } from '../syntax/handlers.js';
import { createKaomojiCommandPlugin } from '../syntax/plugins/command-kaomoji.js';
import { createGifCommandPlugin, buildGifMarkdown } from '../syntax/plugins/command-gif.js';
import { createMentionPlugin } from '../syntax/plugins/mention.js';

test('detects mention and command contexts at caret', () => {
	const mention = new MentionHandler({ searchProfiles: async () => [] });
	const command = new SlashCommandHandler({
		key: 'gif',
		trigger: 'gif',
		search: async () => ({})
	});

	assert.deepEqual(mention.detect({ value: 'hello @cut', caret: 10 }), {
		key: 'mention',
		query: 'cut',
		start: 6,
		caret: 10
	});
	assert.deepEqual(command.detect({ value: '/gif cats', caret: 9 }), {
		key: 'gif',
		query: 'cats',
		start: 0,
		caret: 9,
		replaceEnd: 9,
		trigger: 'gif'
	});
});

test('mention plugin applies username insertion', () => {
	const plugin = createMentionPlugin({ searchProfiles: async () => [] });
	const result = plugin.apply(
		{
			value: 'hello @cut',
			start: 6,
			caret: 10
		},
		{ username: 'cutiecube' }
	);
	assert.equal(result.value, 'hello @cutiecube ');
});

test('kaomoji suggestion and gif insertion replace command range', () => {
	const kaoPlugin = createKaomojiCommandPlugin({
		searchKaomojis: async () => ({ items: [], suggestions: [], error: null })
	});
	const gifPlugin = createGifCommandPlugin({
		searchGifs: async () => ({ items: [], suggestions: [], error: null })
	});

	const suggestion = kaoPlugin.applySuggestion(
		{ value: '/kao jo', start: 0, replaceEnd: 7 },
		'joy'
	);
	assert.equal(suggestion.value, '/kao joy');

	const gif = {
		title: 'happy cat',
		images: {
			original: { url: 'https://gif.test/original.gif' },
			fixed_height: { url: 'https://gif.test/thumb.gif' }
		}
	};
	assert.equal(buildGifMarkdown(gif), '![happy cat](https://gif.test/original.gif)');

	const gifResult = gifPlugin.apply(
		{ value: '/gif cat', start: 0, replaceEnd: 8 },
		gif
	);
	assert.equal(gifResult.value, '![happy cat](https://gif.test/original.gif) ');
});
