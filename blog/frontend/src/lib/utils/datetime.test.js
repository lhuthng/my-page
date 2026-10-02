import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
	formatDateOnly,
	formatDateTime,
	formatLastUpdated,
	formatRelative,
	parseDbDateTime,
	toDateTimeAttr
} from './datetime.js';

const NOW = Date.parse('2026-09-29T21:04:00Z');

test('parseDbDateTime reads the SQLite form as UTC, not local time', () => {
	// The string carries no zone marker; reading it as local time would shift
	// it by the runner's offset.
	assert.equal(parseDbDateTime('2026-09-29 21:04:33').toISOString(), '2026-09-29T21:04:33.000Z');
	assert.equal(parseDbDateTime('2026-09-29T21:04:33').toISOString(), '2026-09-29T21:04:33.000Z');
	assert.equal(parseDbDateTime('2026-09-29 21:04').toISOString(), '2026-09-29T21:04:00.000Z');
});

test('parseDbDateTime leaves an explicit zone alone', () => {
	assert.equal(
		parseDbDateTime('2026-09-29T21:04:33+02:00').toISOString(),
		'2026-09-29T19:04:33.000Z'
	);
	assert.equal(parseDbDateTime('2026-09-29T21:04:33Z').toISOString(), '2026-09-29T21:04:33.000Z');
});
test('parseDbDateTime accepts unix seconds and rejects nonsense', () => {
	assert.equal(
		parseDbDateTime(1793301600).toISOString(),
		new Date(1793301600 * 1000).toISOString()
	);
	assert.equal(parseDbDateTime('not a date'), null);
	assert.equal(parseDbDateTime(''), null);
	assert.equal(parseDbDateTime('   '), null);
	assert.equal(parseDbDateTime(null), null);
	assert.equal(parseDbDateTime(undefined), null);
	assert.equal(parseDbDateTime(NaN), null);
});

test('parseDbDateTime passes a Date through and rejects an invalid one', () => {
	// A caller timing something in this session already holds a real Date.
	const date = new Date('2026-09-29T21:04:33Z');
	assert.equal(parseDbDateTime(date), date);
	assert.equal(parseDbDateTime(new Date('nonsense')), null);
});

test('formatRelative steps through the thresholds', () => {
	const ago = (seconds) => formatRelative(new Date(NOW - seconds * 1000).toISOString(), NOW);
	assert.equal(ago(5), 'just now');
	assert.equal(ago(44), 'just now');
	assert.equal(ago(120), '2 min ago');
	assert.equal(ago(3600), '1 hour ago');
	assert.equal(ago(3600 * 5), '5 hours ago');
	assert.equal(ago(86400), '1 day ago');
	assert.equal(ago(86400 * 4), '4 days ago');
	assert.equal(ago(86400 * 45), '2 months ago');
	assert.equal(ago(86400 * 400), '1 year ago');
	assert.equal(ago(86400 * 800), '2 years ago');
});

test('formatRelative treats a future timestamp as brand new', () => {
	// Clock skew should not render as "-3 minutes ago".
	assert.equal(formatRelative(new Date(NOW + 60000).toISOString(), NOW), 'just now');
});

test('formatRelative is empty for an absent timestamp', () => {
	assert.equal(formatRelative(null), '');
	assert.equal(formatRelative('nope'), '');
});

test('formatDateTime shows both the date and the time', () => {
	const formatted = formatDateTime('2026-09-29 21:04:33', { locale: 'en' });
	assert.match(formatted, /29/);
	assert.match(formatted, /2026/);
	// Rendered in the reader's own zone, so the clock value is not fixed — but
	// a date with no time in it at all would defeat the point of the field.
	assert.match(formatted, /\d{1,2}:\d{2}\s*(am|pm)?/);
});

test('formatDateTime has a long style and is empty for absent values', () => {
	assert.match(formatDateTime('2026-09-29 21:04:33', { style: 'long' }), /September/);
	assert.equal(formatDateTime(null), '');
	assert.equal(formatDateTime(''), '');
});

test('formatLastUpdated pairs the relative age with the absolute moment', () => {
	const recent = formatLastUpdated(new Date(NOW - 3600 * 1000).toISOString(), { now: NOW });
	assert.match(recent, /^1 hour ago · /);
	assert.match(recent, /\d{1,2}:\d{2}\s*(am|pm)?/);
	assert.match(recent, /2026/);
});

test('formatLastUpdated drops the relative half past a week', () => {
	// "3 months ago · 29 Jun 2026, 21:04" is noise; the date alone says it.
	const old = formatLastUpdated(new Date(NOW - 86400 * 40 * 1000).toISOString(), { now: NOW });
	assert.ok(!old.includes('ago'));
	assert.match(old, /2026/);
});
test('formatDateOnly reports the day the database stored, not the reader’s', () => {
	// The whole point: this is a *record's* day, so it must not depend on where
	// the reader is. 23:30 UTC is already the next calendar day in Hanoi and
	// still the previous evening in New York — neither may move the date.
	assert.equal(formatDateOnly('2026-09-29 23:30:00'), 'Sep 29, 2026');
	assert.equal(formatDateOnly('2026-09-30 00:30:00'), 'Sep 30, 2026');
	// The old implementation rendered this in local time, so a reader west of
	// Greenwich was shown the 28th.
	assert.equal(formatDateOnly('2026-09-29 00:30:00'), 'Sep 29, 2026');
});

test('formatDateOnly handles a date-only value and absent input', () => {
	assert.equal(formatDateOnly('2026-09-29'), 'Sep 29, 2026');
	assert.equal(formatDateOnly(new Date('2026-09-29T12:00:00Z')), 'Sep 29, 2026');
	assert.equal(formatDateOnly(null), '');
	assert.equal(formatDateOnly(''), '');
	assert.equal(formatDateOnly('nonsense'), '');
});

test('formatDateOnly can opt into the reader’s zone', () => {
	// Used by `nowToDate`, where the question really is "what day is it for the
	// author here" rather than "what day is this record on".
	const lateUtc = new Date('2026-09-29T23:30:00Z');
	assert.equal(formatDateOnly(lateUtc, { timeZone: 'Asia/Bangkok' }), 'Sep 30, 2026');
	assert.equal(formatDateOnly(lateUtc, { timeZone: 'America/New_York' }), 'Sep 29, 2026');
	assert.equal(formatDateOnly(lateUtc), 'Sep 29, 2026');
});

test('formatLastUpdated and toDateTimeAttr are empty for absent values', () => {
	assert.equal(formatLastUpdated(null), '');
	assert.equal(toDateTimeAttr(null), '');
	assert.equal(toDateTimeAttr('bad'), '');
	assert.equal(toDateTimeAttr('2026-09-29 21:04:33'), '2026-09-29T21:04:33.000Z');
});
