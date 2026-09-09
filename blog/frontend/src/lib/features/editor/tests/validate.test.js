import { test } from 'node:test';
import assert from 'node:assert/strict';
import { validateBasics, validatePatchFields } from '../model/validate.js';

test('validateBasics rejects empty, blank, and oversized fields in order', () => {
	assert.equal(validateBasics({ title: 'T', slug: 'ok', excerpt: 'E' }), null);
	assert.match(validateBasics({ title: '', slug: 'ok', excerpt: 'E' }), /Title/);
	assert.match(validateBasics({ title: '   ', slug: 'ok', excerpt: 'E' }), /Title/);
	assert.match(validateBasics({ title: 'T', slug: '', excerpt: 'E' }), /Slug/);
	assert.match(validateBasics({ title: 'T', slug: 'ok', excerpt: '' }), /Excerpt/);
	assert.match(validateBasics({ title: 'x'.repeat(201), slug: 'ok', excerpt: 'E' }), /Title/);
	assert.match(validateBasics({ title: 'T', slug: 'ok', excerpt: 'x'.repeat(401) }), /Excerpt/);
});

test('validateBasics slug rules mirror the backend allowlist', () => {
	for (const bad of ['a', '-', 'A B', 'über', 'a/b', 'ok slug'])
		assert.match(String(validateBasics({ title: 'T', slug: bad, excerpt: 'E' })), /Slug/);
	for (const good of ['ok', 'hello-world_1', 'abc123'])
		assert.equal(validateBasics({ title: 'T', slug: good, excerpt: 'E' }), null);
});

test('validatePatchFields only validates what the patch touches', () => {
	assert.equal(validatePatchFields({}), null);
	assert.equal(validatePatchFields({ title: 'New title' }), null);
	assert.match(validatePatchFields({ title: '   ' }), /Title/);
	assert.match(validatePatchFields({ slug: 'A B' }), /Slug/);
	assert.match(validatePatchFields({ excerpt: '  ' }), /Excerpt/);
	// full fields absent from the patch never fail it
	assert.equal(validatePatchFields({ title: 'New' }), null);
});

test('trim boundaries pass', () => {
	assert.equal(validateBasics({ title: '  Hi  ', slug: ' ok ', excerpt: ' Yo ' }), null);
});
