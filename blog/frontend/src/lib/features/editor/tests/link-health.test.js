import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createMarkdownRenderer, renderBody } from '../markdown/renderer.js';

const moved = {
	ref_key: 'content:1',
	target_kind: 'content',
	entity_kind: 'project',
	target_slug: 'old-project',
	current_slug: 'new-project',
	status: 'moved'
};

const missing = {
	ref_key: 'content:2',
	target_kind: 'content',
	entity_kind: 'game',
	target_slug: 'gone-game',
	current_slug: null,
	status: 'missing'
};

test('a moved link is rewritten to the current slug', () => {
	const html = renderBody(createMarkdownRenderer(), '[see](/projects/old-project)', {}, [moved]);
	assert.match(html, /href="\/projects\/new-project"/);
	assert.match(html, /data-link-moved="true"/);
});

test('a moved link is rewritten even when the body used an absolute URL', () => {
	const html = renderBody(
		createMarkdownRenderer(),
		'[see](https://blog.huuthangle.site/projects/old-project)',
		{},
		[moved]
	);
	assert.match(html, /href="\/projects\/new-project"/);
});

test('a missing target keeps its href but is marked', () => {
	const html = renderBody(createMarkdownRenderer(), '[see](/games/gone-game)', {}, [missing]);
	assert.match(html, /href="\/games\/gone-game"/);
	assert.match(html, /class="missing-link"/);
	assert.match(html, /data-link-missing="game"/);
	assert.match(html, /This game is no longer available\./);
});

test('healthy and unrelated links are left alone', () => {
	const html = renderBody(
		createMarkdownRenderer(),
		'[ok](/posts/fine) [out](https://example.com)',
		{},
		[{ ...moved, target_slug: 'fine', current_slug: 'fine', status: 'ok' }]
	);
	assert.match(html, /href="\/posts\/fine"/);
	assert.doesNotMatch(html, /missing-link/);
	assert.doesNotMatch(html, /data-link-moved/);
	assert.match(html, /href="https:\/\/example.com"/);
});

test('series links use their own target kind', () => {
	const series = {
		ref_key: 'series:9',
		target_kind: 'series',
		entity_kind: 'series',
		target_slug: 'old-series',
		current_slug: 'new-series',
		status: 'moved'
	};
	const post = {
		...series,
		ref_key: 'content:9',
		target_kind: 'content',
		entity_kind: 'post',
		target_slug: 'old-series',
		current_slug: 'wrong'
	};
	const html = renderBody(createMarkdownRenderer(), '[s](/series/old-series)', {}, [post, series]);
	assert.match(html, /href="\/series\/new-series"/);
});

test('a moved app embed follows the project to its new slug', () => {
	const html = renderBody(createMarkdownRenderer(), ':::app project old-project 100% 400', {}, [
		moved
	]);
	assert.match(html, /data-name="new-project"/);
	assert.doesNotMatch(html, /data-missing/);
});

test('a missing app embed is marked for the frontend to render as gone', () => {
	const html = renderBody(createMarkdownRenderer(), ':::app game gone-game 100% 400', {}, [
		missing
	]);
	assert.match(html, /data-name="gone-game"/);
	assert.match(html, /data-missing="true"/);
});

test('media embeds are not affected by content link health', () => {
	const html = renderBody(createMarkdownRenderer(), ':::app glb-demo model', {}, [moved, missing]);
	assert.match(html, /data-name="model"/);
	assert.doesNotMatch(html, /data-missing/);
});

test('a missing media reference says so instead of showing the key', () => {
	const html = renderBody(createMarkdownRenderer(), '@[img:0]', {}, []);
	assert.match(html, /class="missing-image"/);
	assert.match(html, /Missing media/);
	assert.doesNotMatch(html, />0</);
});
