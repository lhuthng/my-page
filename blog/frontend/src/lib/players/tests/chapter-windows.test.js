import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
	CHAPTER_WINDOW,
	missingWindows,
	planRows,
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
	// A negative index is the head of the book, not a window before it.
	assert.equal(windowStart(-4), 0);
});

test('windowStart honours a custom window size', () => {
	assert.equal(windowStart(9, 5), 5);
	assert.equal(windowStart(10, 5), 10);
});

test('windowStarts covers the book and stops at the last chapter', () => {
	assert.deepEqual(windowStarts(0), []);
	assert.deepEqual(windowStarts(1), [0]);
	assert.deepEqual(windowStarts(CHAPTER_WINDOW), [0]);
	assert.deepEqual(windowStarts(CHAPTER_WINDOW + 1), [0, CHAPTER_WINDOW]);
	assert.deepEqual(windowStarts(5, 5), [0]);
	assert.deepEqual(windowStarts(6, 5), [0, 5]);
});

test('windowCount clamps the last window to the end of the book', () => {
	assert.equal(windowCount(0, 45), 20);
	assert.equal(windowCount(40, 45), 5);
	assert.equal(windowCount(60, 45), 0);
});

test('planRows paints loaded chapters and nothing else when the book is loaded', () => {
	const rows = planRows({ total: 3, loaded: [0] });
	assert.deepEqual(rows, [
		{ kind: 'chapter', index: 0 },
		{ kind: 'chapter', index: 1 },
		{ kind: 'chapter', index: 2 }
	]);
});

test('planRows renders one skeleton per window in flight', () => {
	const rows = planRows({ total: 45, loaded: [0], pending: [20] });
	assert.deepEqual(rows, [
		...Array.from({ length: 20 }, (_, index) => ({ kind: 'chapter', index })),
		{ kind: 'skeleton', start: 20, count: 20 },
		{ kind: 'gap', start: 40, count: 5 }
	]);
});

test('planRows keeps a book of many chapters to a handful of items', () => {
	const rows = planRows({ total: 2000, loaded: [0], pending: [20] });
	assert.equal(rows.length, 22);
	assert.equal(rows.at(-1).kind, 'gap');
	assert.equal(rows.at(-1).count, 1980 - 20);
	// The gap is reachable: it carries the window that fetches it.
	assert.equal(rows.at(-1).start, 40);
});

test('planRows describes a book resumed far from the start in one gap row', () => {
	// Window 0 came with the page and window 300 was fetched for the saved
	// position: everything between them is a single reachable row, not 280 rows
	// of skeleton for chapters the reader never asked for.
	const rows = planRows({ total: 320, loaded: [0, 300], pending: [] });
	assert.equal(rows.length, 41);
	assert.deepEqual(rows[19], { kind: 'chapter', index: 19 });
	assert.deepEqual(rows[20], { kind: 'gap', start: 20, count: 280 });
	assert.deepEqual(rows[21], { kind: 'chapter', index: 300 });
	assert.deepEqual(rows.at(-1), { kind: 'chapter', index: 319 });
});

test('planRows renders nothing for a book with no chapters', () => {
	assert.deepEqual(planRows({ total: 0, loaded: [0] }), []);
});

test('missingWindows names the windows a full read still needs', () => {
	assert.deepEqual(missingWindows(45, [0]), [20, 40]);
	assert.deepEqual(missingWindows(45, [0, 20, 40]), []);
	assert.deepEqual(missingWindows(20, []), [0]);
});
