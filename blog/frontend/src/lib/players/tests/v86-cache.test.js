import { test } from 'node:test';
import assert from 'node:assert/strict';
import { chunkPrefix, chunkUrl, formatBytes } from '../v86-cache.js';

test('chunkUrl mirrors the backend part naming, including the last short chunk', () => {
	const prefix = '/api/games/s/demo/v86/disk/sha/';
	assert.equal(chunkUrl(prefix, 0, 262144), '/api/games/s/demo/v86/disk/sha/0-262144.img.zst');
	// step stays chunk-size even past the end of the image
	assert.equal(
		chunkUrl(prefix, 3, 262144),
		'/api/games/s/demo/v86/disk/sha/786432-1048576.img.zst'
	);
	assert.equal(chunkPrefix('/api/x/6bf9b/.img.zst'), '/api/x/6bf9b/');
});

test('formatBytes speaks human units', () => {
	assert.equal(formatBytes(0), '0 MB');
	assert.equal(formatBytes(262144), '256 KB');
	assert.equal(formatBytes(240 * 1048576), '240 MB');
	assert.equal(formatBytes(1.5 * 1073741824), '1.50 GB');
});
