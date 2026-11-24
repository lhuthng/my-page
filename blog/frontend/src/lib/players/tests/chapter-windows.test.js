import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
	CHAPTER_WINDOW,
	clampPage,
	missingWindows,
	pageCount,
	pageStart,
	parseChapterNumber,
	validateChapterJump,
	windowCount,
	windowStart,
	windowStarts
} from '../chapter-windows.js';

test('windowStart rounds an index down to its window', () => {
	assert.equal(windowStart(0), 0);
	assert.equal(windowStart(19), 0);
	assert.equal(windowStart(20), 20);
	assert.equal(windowStart(21), 20);
	assert.equal(windowStart(999), 980);
	assert.equal(windowStart(-4), 0);
	assert.equal(windowStart(10, 5), 10);
});

test('windowStarts covers the book and stops at the last chapter', () => {
	assert.deepEqual(windowStarts(0), []);
	assert.deepEqual(windowStarts(1), [0]);
	assert.deepEqual(windowStarts(CHAPTER_WINDOW), [0]);
	assert.deepEqual(windowStarts(CHAPTER_WINDOW + 1), [0, CHAPTER_WINDOW]);
	assert.deepEqual(windowStarts(6, 5), [0, 5]);
});

test('windowCount clamps the last window to the end of the book', () => {
	assert.equal(windowCount(0, 45), 20);
	assert.equal(windowCount(40, 45), 5);
	assert.equal(windowCount(60, 45), 0);
});

test('page navigation handles a long book and its short final page', () => {
	assert.equal(pageCount(1152), 58);
	assert.equal(pageStart(53), 1040);
	assert.equal(windowCount(pageStart(58), 1152), 12);
	assert.equal(pageCount(0), 0);
});

test('clampPage keeps navigation inside the available pages', () => {
	assert.equal(clampPage(0, 1152), 1);
	assert.equal(clampPage(-2, 1152), 1);
	assert.equal(clampPage(99, 1152), 58);
	assert.equal(clampPage(3.9, 1152), 3);
	assert.equal(clampPage(Number.NaN, 1152), 1);
	assert.equal(clampPage(4, 0), 1);
});

test('parseChapterNumber accepts only positive safe integers', () => {
	assert.equal(parseChapterNumber('1050'), 1050);
	assert.equal(parseChapterNumber(' 001050 '), 1050);
	for (const value of ['', '0', '-1', '+1', '1.5', '1e3', '1050x', '9007199254740992']) {
		assert.equal(parseChapterNumber(value), null, value);
	}
});

test('validateChapterJump derives the ordinal from the returned aligned window', () => {
	const audiobook = {
		tracks_offset: 1040,
		track_count: 1152,
		tracks: Array.from({ length: 20 }, (_, index) => ({ number: 1041 + index }))
	};
	assert.deepEqual(validateChapterJump(audiobook, 1050), {
		offset: 1040,
		ordinal: 1049,
		total: 1152
	});
});

test('validateChapterJump supports gapped chapter numbers without guessing offsets', () => {
	const audiobook = {
		tracks_offset: 20,
		track_count: 25,
		tracks: [
			{ number: 500 },
			{ number: 1050 },
			{ number: 2000 },
			{ number: 2001 },
			{ number: 2002 }
		]
	};
	assert.deepEqual(validateChapterJump(audiobook, 1050), {
		offset: 20,
		ordinal: 21,
		total: 25
	});
});

test('validateChapterJump rejects incomplete or inconsistent answers', () => {
	assert.equal(validateChapterJump(null, 1050), null);
	assert.equal(
		validateChapterJump({ tracks_offset: 21, track_count: 30, tracks: [{ number: 1050 }] }, 1050),
		null
	);
	assert.equal(
		validateChapterJump({ tracks_offset: 20, track_count: 20, tracks: [{ number: 1050 }] }, 1050),
		null
	);
	assert.equal(
		validateChapterJump({ tracks_offset: 20, track_count: 30, tracks: [{ number: 1049 }] }, 1050),
		null
	);
	assert.equal(
		validateChapterJump({ tracks_offset: 20, track_count: 30, tracks: [{ number: 1050 }] }, 1050),
		null,
		'a truncated window cannot prove the target ordinal'
	);
	assert.equal(
		validateChapterJump(
			{
				tracks_offset: 20,
				track_count: 30,
				tracks: Array.from({ length: 21 }, (_, index) => ({ number: index + 1 }))
			},
			1050
		),
		null
	);
});

test('missingWindows names the windows a full read still needs', () => {
	assert.deepEqual(missingWindows(45, [0]), [20, 40]);
	assert.deepEqual(missingWindows(45, [0, 20, 40]), []);
	assert.deepEqual(missingWindows(20, []), [0]);
});
