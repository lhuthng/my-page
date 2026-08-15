import test from 'node:test';
import assert from 'node:assert/strict';
import { buildPatch, isPatchEmpty, normalizeLinks, splitTags } from '../model/diff.js';

// `baseline.tags` mirrors the server response shape: an array. `current.tags`
// mirrors the editor's textarea binding: a space-joined string, split by
// `buildPatch` via `splitTags` before comparing. Mixing the two up here would
// silently pass every test, since JS lets you `.sort()` a string's char array
// too — so the fixture keeps them intentionally distinct, the way the real
// baseline/current pairing does.
function baselineState(overrides = {}) {
	return {
		title: 'Title',
		slug: 'title',
		excerpt: 'Excerpt',
		tags: ['a', 'b'],
		bodies: { draft: 'draft body', content: 'published body' },
		ogImageSeconds: 0,
		...overrides
	};
}

function postState(overrides = {}) {
	return {
		title: 'Title',
		slug: 'title',
		excerpt: 'Excerpt',
		tags: 'a b',
		bodies: { draft: 'draft body', content: 'published body' },
		ogImageSeconds: 0,
		...overrides
	};
}

test('splitTags trims, splits on whitespace, and drops blanks', () => {
	assert.deepEqual(splitTags('  a   b  c '), ['a', 'b', 'c']);
	assert.deepEqual(splitTags(''), []);
	assert.deepEqual(splitTags(undefined), []);
});

test('normalizeLinks drops incomplete pairs and trims the rest', () => {
	assert.deepEqual(
		normalizeLinks([
			{ label: '  GitHub  ', url: ' https://x ' },
			{ label: '', url: 'https://y' },
			{ label: 'no url', url: '' }
		]),
		[{ label: 'GitHub', url: 'https://x' }]
	);
	assert.deepEqual(normalizeLinks(undefined), []);
});

test('an unchanged state produces an empty patch', () => {
	const patch = buildPatch({ baseline: baselineState(), current: postState() });
	assert.ok(isPatchEmpty(patch), JSON.stringify(patch));
});

test('only the fields that actually changed are included', () => {
	const baseline = baselineState();
	const current = postState({ title: 'New title' });
	const patch = buildPatch({ baseline, current });
	assert.deepEqual(patch, { title: 'New title' });
});

test('tags are compared order-independently but sent as the new order', () => {
	const baseline = baselineState({ tags: ['a', 'b'] });
	const current = postState({ tags: 'b a' });
	// Same set, different order: not a change.
	assert.ok(isPatchEmpty(buildPatch({ baseline, current })));

	const changed = postState({ tags: 'a b c' });
	assert.deepEqual(buildPatch({ baseline, current: changed }), { tags: ['a', 'b', 'c'] });
});

test('draft and content are sent together when the draft text changed', () => {
	const baseline = baselineState();
	const current = postState({ bodies: { draft: 'edited', content: 'published body' } });
	assert.deepEqual(buildPatch({ baseline, current }), {
		draft: 'edited',
		content: 'published body'
	});
});

test('draft and content are sent when only newly-uploaded media forces it', () => {
	const baseline = baselineState();
	const current = postState();
	const patch = buildPatch({ baseline, current, hasNewMedia: true });
	assert.deepEqual(patch, { draft: 'draft body', content: 'published body' });
});

test('an unchanged draft with no new media sends neither field', () => {
	const patch = buildPatch({ baseline: baselineState(), current: postState() });
	assert.equal('draft' in patch, false);
	assert.equal('content' in patch, false);
});

test('og_image_seconds only appears when it actually differs, treating missing as 0', () => {
	const baseline = baselineState({ ogImageSeconds: undefined });
	assert.ok(isPatchEmpty(buildPatch({ baseline, current: postState({ ogImageSeconds: 0 }) })));
	assert.deepEqual(buildPatch({ baseline, current: postState({ ogImageSeconds: 5 }) }), {
		og_image_seconds: 5
	});
});

test('saving twice in a row produces an empty second patch once the baseline is refreshed', () => {
	const baseline = baselineState();
	const current = postState({ title: 'Edited once' });

	const firstPatch = buildPatch({ baseline, current });
	assert.deepEqual(firstPatch, { title: 'Edited once' });

	// The caller refreshes its baseline from the server's response after a
	// successful save — simulate that by folding the patch into a fresh
	// baseline (tags normalized to an array, the way the server echoes them
	// back), then diffing the same (unedited-since) current state again.
	const refreshedBaseline = { ...baseline, ...current, tags: ['a', 'b'] };
	const secondPatch = buildPatch({ baseline: refreshedBaseline, current });
	assert.ok(isPatchEmpty(secondPatch), JSON.stringify(secondPatch));
});

function projectBaseline(overrides = {}) {
	return {
		...baselineState(),
		demoType: 'html5',
		demoWidth: '100%',
		demoHeight: '520px',
		demoUrl: '',
		links: [{ label: 'GitHub', url: 'https://github.com/x' }],
		...overrides
	};
}

function projectState(overrides = {}) {
	return {
		...postState(),
		demoType: 'html5',
		demoWidth: '100%',
		demoHeight: '520px',
		demoUrl: '',
		links: [{ label: 'GitHub', url: 'https://github.com/x' }],
		...overrides
	};
}

test('project patches are empty for defaults expressed as missing baseline fields', () => {
	const baseline = {
		...baselineState(),
		demoType: undefined,
		demoWidth: undefined,
		demoHeight: undefined,
		demoUrl: undefined,
		links: undefined
	};
	const current = projectState();
	const patch = buildPatch({ baseline, current, kind: 'project' });
	// demoType/demoWidth/demoHeight/demoUrl all equal their implied defaults.
	assert.equal('demo_type' in patch, false);
	assert.equal('demo_width' in patch, false);
	assert.equal('demo_height' in patch, false);
	assert.equal('demo_url' in patch, false);
	// links differ from an empty baseline.
	assert.deepEqual(patch.links, [{ label: 'GitHub', url: 'https://github.com/x' }]);
});

test('switching a project to none clears a previously-set demo url', () => {
	const baseline = projectBaseline({ demoType: 'embed', demoUrl: 'https://old.example' });
	const current = projectState({ demoType: 'none', demoUrl: '' });
	const patch = buildPatch({ baseline, current, kind: 'project' });
	assert.equal(patch.demo_type, 'none');
	assert.equal(patch.demo_url, '');
});

test('a demo url change is only sent for url-based demo types', () => {
	const baseline = projectBaseline({ demoType: 'embed', demoUrl: 'https://old.example' });
	// html5/webgl/v86 never carry a demo_url — the field is meaningless there.
	const htmlCurrent = projectState({ demoType: 'html5', demoUrl: 'https://old.example' });
	assert.equal(
		'demo_url' in buildPatch({ baseline, current: htmlCurrent, kind: 'project' }),
		false
	);

	const embedCurrent = projectState({ demoType: 'embed', demoUrl: 'https://new.example' });
	assert.equal(
		buildPatch({ baseline, current: embedCurrent, kind: 'project' }).demo_url,
		'https://new.example'
	);
});

test('project links diff by content, not identity', () => {
	const baseline = projectBaseline();
	const same = projectState({ links: [{ label: 'GitHub', url: 'https://github.com/x' }] });
	assert.ok(isPatchEmpty(buildPatch({ baseline, current: same, kind: 'project' })));

	const changed = projectState({ links: [{ label: 'GitHub', url: 'https://github.com/y' }] });
	assert.deepEqual(buildPatch({ baseline, current: changed, kind: 'project' }).links, [
		{ label: 'GitHub', url: 'https://github.com/y' }
	]);
});

test('an unset links placeholder on both sides is not a diff', () => {
	// `loadEntryState` substitutes `[{label:'GitHub',url:''}]` for an empty
	// links list on *both* baseline and entry. Comparing a normalized
	// `current.links` (which drops that blank-url placeholder) against a raw
	// `baseline.links` (which still has it) reported a spurious diff — every
	// project with no real links loaded as permanently dirty.
	const placeholder = [{ label: 'GitHub', url: '' }];
	const baseline = projectBaseline({ links: placeholder });
	const current = projectState({ links: placeholder });
	assert.ok(
		isPatchEmpty(buildPatch({ baseline, current, kind: 'project' })),
		'an untouched placeholder link must not count as a change'
	);
});
