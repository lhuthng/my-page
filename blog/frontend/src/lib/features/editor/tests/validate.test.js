import { test } from 'node:test';
import assert from 'node:assert/strict';
import { validateBasicsFields, validatePatchFieldsMap } from '../model/validate.js';

test('validateBasicsFields keys errors by the field that caused them', () => {
	assert.deepEqual(validateBasicsFields({ title: 'T', slug: 'ok', excerpt: 'E' }), {});
	assert.deepEqual(Object.keys(validateBasicsFields({ title: '', slug: 'ok', excerpt: 'E' })), ['title']);
	assert.deepEqual(Object.keys(validateBasicsFields({ title: '   ', slug: 'ok', excerpt: 'E' })), [
		'title'
	]);
	assert.deepEqual(Object.keys(validateBasicsFields({ title: 'T', slug: '', excerpt: 'E' })), ['slug']);
	assert.deepEqual(Object.keys(validateBasicsFields({ title: 'T', slug: 'ok', excerpt: '' })), [
		'excerpt'
	]);
	assert.deepEqual(
		Object.keys(validateBasicsFields({ title: 'x'.repeat(201), slug: 'ok', excerpt: 'E' })),
		['title']
	);
	assert.deepEqual(
		Object.keys(validateBasicsFields({ title: 'T', slug: 'ok', excerpt: 'x'.repeat(401) })),
		['excerpt']
	);
});

test('every bad field is reported at once, not just the first', () => {
	// The whole point of the field-keyed shape. A validator that returns one
	// message at a time makes the user submit three times to learn three
	// things, and gives the editor nothing to render under each input.
	assert.deepEqual(Object.keys(validateBasicsFields({ title: '', slug: '', excerpt: '' })), [
		'title',
		'slug',
		'excerpt'
	]);
});

test('slug rules mirror the backend allowlist', () => {
	for (const bad of ['a', '-', 'A B', 'über', 'a/b', 'ok slug'])
		assert.ok(validateBasicsFields({ title: 'T', slug: bad, excerpt: 'E' }).slug, bad);
	for (const good of ['ok', 'hello-world_1', 'abc123'])
		assert.deepEqual(validateBasicsFields({ title: 'T', slug: good, excerpt: 'E' }), {});
});

test('validatePatchFieldsMap only validates what the patch touches', () => {
	assert.deepEqual(validatePatchFieldsMap({}), {});
	assert.deepEqual(validatePatchFieldsMap({ title: 'New title' }), {});
	assert.ok(validatePatchFieldsMap({ title: '   ' }).title);
	assert.ok(validatePatchFieldsMap({ slug: 'A B' }).slug);
	assert.ok(validatePatchFieldsMap({ excerpt: '  ' }).excerpt);
});

test('a patch that omits a field cannot fail on it', () => {
	// The edit flow sends only what changed, so an untouched (and possibly
	// empty) excerpt must not block a title-only save.
	const errors = validatePatchFieldsMap({ title: 'New' });
	assert.equal('excerpt' in errors, false);
	assert.equal('slug' in errors, false);
});

test('trim boundaries pass', () => {
	assert.deepEqual(validateBasicsFields({ title: '  Hi  ', slug: ' ok ', excerpt: ' Yo ' }), {});
});
