import test from 'node:test';
import assert from 'node:assert/strict';
import { createEntryState, loadEntryState, refreshBaseline } from '../model/state.js';
import { buildPatch, isPatchEmpty } from '../model/diff.js';

test('createEntryState seeds a blank post', () => {
	const entry = createEntryState('post', [{ id: 1, title: 'Series A' }]);
	assert.equal(entry.id, '');
	assert.equal(entry.tags, '');
	assert.deepEqual(entry.bodies, { draft: '', content: '' });
	assert.deepEqual(entry.series, [{ id: 1, title: 'Series A' }]);
	assert.equal(entry.pendingSeriesId, null);
});

test('createEntryState seeds a blank project with the demo defaults', () => {
	const entry = createEntryState('project');
	assert.equal(entry.demoType, 'html5');
	assert.equal(entry.demoWidth, '100%');
	assert.deepEqual(entry.links, [{ label: 'GitHub', url: '' }]);
});

function loadedPost(overrides = {}) {
	return {
		id: 5,
		title: 'Hello',
		slug: 'hello',
		excerpt: 'An excerpt',
		tags: ['rust', 'sqlite'],
		content: 'published body',
		draft: 'draft body',
		coverUrl: '/cover.png',
		cover_media_type: 'image/png',
		series: [],
		seriesSlug: '',
		relatedPosts: [],
		updatedAt: '2026-08-15 10:00:00',
		...overrides
	};
}

test('loadEntryState maps a post into an editable entry and a diffable baseline', () => {
	const { entry, baseline } = loadEntryState(loadedPost(), 'post');

	// entry: editable shape.
	assert.equal(entry.tags, 'rust sqlite');
	assert.deepEqual(entry.bodies, { draft: 'draft body', content: 'published body' });
	assert.equal(entry.pendingSeriesId, null);

	// baseline: diffable shape (tags stay an array; carries the lock token).
	assert.deepEqual(baseline.tags, ['rust', 'sqlite']);
	assert.equal(baseline.updatedAt, '2026-08-15 10:00:00');

	// A fresh load against itself must diff to nothing.
	assert.ok(isPatchEmpty(buildPatch({ baseline, current: entry })));
});

test('loadEntryState maps a project including demo fields', () => {
	const data = {
		...loadedPost(),
		postId: 9,
		demoType: 'embed',
		demoWidth: '80%',
		demoHeight: '400px',
		rawDemoUrl: 'https://example.com/demo',
		delegateGameId: 3,
		inheritThumbnail: false,
		inheritTags: true,
		links: [{ label: 'Repo', url: 'https://github.com/x' }]
	};
	const { entry, baseline } = loadEntryState(data, 'project');

	assert.equal(entry.demoType, 'embed');
	assert.equal(entry.demoUrl, 'https://example.com/demo');
	assert.equal(entry.delegateGameId, '3', 'should be stringified for the <select> binding');
	assert.equal(entry.inheritThumbnail, false);
	assert.deepEqual(entry.links, [{ label: 'Repo', url: 'https://github.com/x' }]);

	assert.ok(isPatchEmpty(buildPatch({ baseline, current: entry, kind: 'project' })));
});

test('loadEntryState maps a game including launcher and body fields', () => {
	const data = {
		...loadedPost(),
		postId: 9,
		demoType: 'v86',
		rawDemoUrl: '',
		v86SystemVersionId: 3,
		v86Manifest: 'exe=a.exe',
		v86ArtifactRevision: 2,
		instruction: 'Arrows to move',
		cheatcode: 'IDDQD',
		story: 'A long story',
		relatedGames: [{ id: 2, title: 'Other', slug: 'other' }]
	};
	const { entry, baseline } = loadEntryState(data, 'game');

	assert.equal(entry.demoType, 'v86');
	assert.equal(entry.v86SystemVersionId, '3', 'should be stringified for the <select> binding');
	assert.equal(entry.instruction, 'Arrows to move');
	assert.deepEqual(entry.relatedGames, [{ id: 2, title: 'Other', slug: 'other' }]);

	assert.ok(isPatchEmpty(buildPatch({ baseline, current: entry, kind: 'game' })));
});

test('loadEntryState defaults an empty links list to the GitHub placeholder', () => {
	const { entry } = loadEntryState({ ...loadedPost(), links: [] }, 'project');
	assert.deepEqual(entry.links, [{ label: 'GitHub', url: '' }]);
});

test('refreshBaseline folds a save back so the next diff is empty', () => {
	const { entry, baseline } = loadEntryState(loadedPost(), 'post');
	const edited = { ...entry, title: 'Edited title', tags: 'rust axum' };

	const patch = buildPatch({ baseline, current: edited });
	assert.deepEqual(patch, { title: 'Edited title', tags: ['rust', 'axum'] });

	const nextBaseline = refreshBaseline(baseline, edited, { updatedAt: '2026-08-15 11:00:00' });
	assert.equal(nextBaseline.updatedAt, '2026-08-15 11:00:00');
	assert.ok(isPatchEmpty(buildPatch({ baseline: nextBaseline, current: edited })));
});

test('refreshBaseline keeps the previous lock token when the server did not send one', () => {
	const { baseline } = loadEntryState(loadedPost(), 'post');
	const next = refreshBaseline(
		baseline,
		{ ...baseline, tags: 'rust sqlite', bodies: baseline.bodies, ogImageSeconds: 0 },
		{}
	);
	assert.equal(next.updatedAt, baseline.updatedAt);
});
