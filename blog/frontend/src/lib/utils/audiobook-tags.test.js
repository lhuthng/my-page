import { test } from 'node:test';
import assert from 'node:assert/strict';

import { VIETNAMESE_TRANSLATED_TAG, isVietnameseTranslation } from './audiobook-tags.js';

test('detects the tag on listing payloads (tag_slugs array)', () => {
	assert.equal(
		isVietnameseTranslation({ tag_slugs: ['classic', VIETNAMESE_TRANSLATED_TAG], tags: [] }),
		true
	);
});

test('detects the tag on detail payloads ({ name, slug } objects)', () => {
	assert.equal(
		isVietnameseTranslation({
			tags: [
				{ name: 'Classic', slug: 'classic' },
				{ name: 'Vietnamese Translated', slug: VIETNAMESE_TRANSLATED_TAG }
			]
		}),
		true
	);
});

test('falls back to string tags when tag_slugs is missing', () => {
	assert.equal(isVietnameseTranslation({ tags: ['classic', VIETNAMESE_TRANSLATED_TAG] }), true);
});

test('returns false for other or missing tags', () => {
	assert.equal(isVietnameseTranslation({ tag_slugs: ['classic'], tags: ['Classic'] }), false);
	assert.equal(isVietnameseTranslation({}), false);
	assert.equal(isVietnameseTranslation(undefined), false);
});
