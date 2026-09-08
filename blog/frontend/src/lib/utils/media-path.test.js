import { test } from 'node:test';
import assert from 'node:assert/strict';
import { normalizeMediaPath } from './media-path.js';

test('normalizeMediaPath strips disk-relative segments', () => {
	assert.equal(normalizeMediaPath('./media/post/2/x.gif'), 'media/post/2/x.gif');
	assert.equal(normalizeMediaPath('media/i/.post.25'), 'media/i/.post.25');
	assert.equal(normalizeMediaPath('./media//a//b.png'), 'media/a/b.png');
	assert.equal(normalizeMediaPath('/media/i/x.png'), 'media/i/x.png');
	assert.equal(normalizeMediaPath('media/i/.post.25.thumbnail'), 'media/i/.post.25.thumbnail');
});
