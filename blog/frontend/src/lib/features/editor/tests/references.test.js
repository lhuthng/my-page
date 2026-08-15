import test from 'node:test';
import assert from 'node:assert/strict';
import { buildMediaDictionary, collectMediaKeys, decodeShortNames } from '../media/references.js';

test('collects keys from every inline media syntax', () => {
	const body = [
		'intro @[img:photo-one] middle',
		'@(200_100)[vid:clip] sized',
		':::app lottie spinner',
		':::app glb-demo model.glb'
	].join('\n');

	assert.deepEqual(collectMediaKeys(body).sort(), ['clip', 'model.glb', 'photo-one', 'spinner']);
});

test('collects across several bodies and deduplicates', () => {
	const keys = collectMediaKeys(['@[img:shared]', '@[vid:shared] @[img:other]']);
	assert.deepEqual(keys.sort(), ['other', 'shared']);
});

test('collecting tolerates empty and missing bodies', () => {
	assert.deepEqual(collectMediaKeys([]), []);
	assert.deepEqual(collectMediaKeys(['', null, undefined]), []);
	assert.deepEqual(collectMediaKeys('no media here'), []);
});

test('decodes stored indices back to short names', () => {
	const stored = 'a @[img:0] b @[vid:1] c';
	assert.equal(decodeShortNames(stored, ['photo', 'clip']), 'a @[img:photo] b @[vid:clip] c');
});

test('decodes right-to-left so earlier offsets stay valid', () => {
	// Replacements of differing length: applying these front-to-back would
	// corrupt every position after the first.
	const stored = '@[img:0] @[img:1] @[img:2]';
	const decoded = decodeShortNames(stored, [
		'a-very-long-short-name-indeed',
		'b',
		'another-long-one'
	]);
	assert.equal(decoded, '@[img:a-very-long-short-name-indeed] @[img:b] @[img:another-long-one]');
});

test('decodes lottie app blocks as well as media shortcuts', () => {
	assert.equal(
		decodeShortNames('@[img:0]\n:::app lottie 1', ['photo', 'spinner']),
		'@[img:photo]\n:::app lottie spinner'
	);
});

test('leaves an index with no matching short name untouched', () => {
	// Blanking the reference would silently corrupt the body.
	assert.equal(decodeShortNames('@[img:0] @[img:9]', ['photo']), '@[img:photo] @[img:9]');
});

test('leaves an already-decoded body untouched', () => {
	const body = '@[img:my-photo] and :::app lottie spinner';
	assert.equal(decodeShortNames(body, ['photo', 'clip']), body);
});

test('does not treat a partially-numeric key as an index', () => {
	const body = '@[img:12abc]';
	assert.equal(decodeShortNames(body, ['zero', 'one', 'two', 'x', 'y', 'z']), body);
});

test('does not decode glb references, which the backend never encodes', () => {
	const body = ':::app glb-demo 0';
	assert.equal(decodeShortNames(body, ['photo']), body);
});

test('decoding tolerates empty input', () => {
	assert.equal(decodeShortNames('', ['a']), '');
	assert.equal(decodeShortNames(undefined, ['a']), '');
	assert.equal(decodeShortNames('@[img:0]', []), '@[img:0]');
	assert.equal(decodeShortNames('@[img:0]', undefined), '@[img:0]');
});

test('builds a dictionary keyed by both index and short name', () => {
	const dict = buildMediaDictionary(['/m/a.png', '/m/b.png'], ['alpha', 'beta']);
	assert.deepEqual(dict, {
		0: '/m/a.png',
		1: '/m/b.png',
		alpha: '/m/a.png',
		beta: '/m/b.png'
	});
});

test('dictionary applies the url resolver to every entry', () => {
	const dict = buildMediaDictionary(['a.png'], ['alpha'], (url) => `/api/${url}`);
	assert.equal(dict['0'], '/api/a.png');
	assert.equal(dict.alpha, '/api/a.png');
});

test('dictionary skips blank short names and handles missing inputs', () => {
	assert.deepEqual(buildMediaDictionary(['a.png'], [null]), { 0: 'a.png' });
	assert.deepEqual(buildMediaDictionary([], ['alpha']), {});
	assert.deepEqual(buildMediaDictionary(undefined, undefined), {});
});

test('decode round-trips against the dictionary the renderer will use', () => {
	const shortNames = ['photo', 'clip'];
	const urls = ['/m/photo.png', '/m/clip.mp4'];
	const decoded = decodeShortNames('@[img:0] @[vid:1]', shortNames);
	const dict = buildMediaDictionary(urls, shortNames);

	for (const key of collectMediaKeys(decoded)) {
		assert.ok(dict[key], `expected a url for ${key}`);
	}
});
