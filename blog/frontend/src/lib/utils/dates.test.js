import { test } from 'node:test';
import assert from 'node:assert/strict';

import { dateTillNow, textToDate } from './index.js';

test('dateTillNow treats the database form and the ISO form as one instant', () => {
	// This equality is the bug fix in one line. The unmarked SQLite string used
	// to be read as local time, so a listener west of Greenwich computed an age
	// that was hours out — and the same record read differently on two machines.
	const fromDb = dateTillNow('2026-09-29 21:04:33');
	const fromIso = dateTillNow('2026-09-29T21:04:33Z');
	assert.equal(fromDb, fromIso);
	assert.notEqual(fromDb, '');
});

test('dateTillNow still honours its format argument', () => {
	const value = '2026-09-29 21:04:33';
	assert.notEqual(dateTillNow(value, 'round'), dateTillNow(value, 'mini'));
});

test('dateTillNow is empty for an absent timestamp', () => {
	// A missing `updated_at` must not throw its way through a page render.
	assert.equal(dateTillNow(null), '');
	assert.equal(dateTillNow(undefined), '');
	assert.equal(dateTillNow(''), '');
	assert.equal(dateTillNow('not a date'), '');
});

test('textToDate returns the stored day and tolerates missing input', () => {
	assert.equal(textToDate('2026-09-29 21:04:33'), 'Sep 29, 2026');
	// Date-only values are what the old `split(' ')[0]` path was really for.
	assert.equal(textToDate('2026-09-29'), 'Sep 29, 2026');
	// The old implementation threw on these, taking the page render with it.
	assert.equal(textToDate(null), '');
	assert.equal(textToDate(undefined), '');
	assert.equal(textToDate(''), '');
});
