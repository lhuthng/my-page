import { test } from 'node:test';
import assert from 'node:assert/strict';

import { formatClock, formatDurationLabel, percentOf } from './duration.js';

test('formatClock renders m:ss under an hour', () => {
	assert.equal(formatClock(0), '0:00');
	assert.equal(formatClock(9), '0:09');
	assert.equal(formatClock(65), '1:05');
	assert.equal(formatClock(3599), '59:59');
});

test('formatClock switches to h:mm:ss at an hour', () => {
	assert.equal(formatClock(3600), '1:00:00');
	assert.equal(formatClock(3725), '1:02:05');
	assert.equal(formatClock(36000), '10:00:00');
});

test('formatClock floors fractional seconds', () => {
	assert.equal(formatClock(12.9), '0:12');
});

test('formatClock returns a placeholder for unknown values', () => {
	assert.equal(formatClock(null), '--:--');
	assert.equal(formatClock(undefined), '--:--');
	assert.equal(formatClock(NaN), '--:--');
	assert.equal(formatClock(Infinity), '--:--');
	assert.equal(formatClock(-5), '--:--');
});

test('formatDurationLabel summarizes totals', () => {
	assert.equal(formatDurationLabel(0), '');
	assert.equal(formatDurationLabel(-1), '');
	assert.equal(formatDurationLabel(NaN), '');
	assert.equal(formatDurationLabel(30), 'under a minute');
	assert.equal(formatDurationLabel(59), 'under a minute');
	assert.equal(formatDurationLabel(60), '1 min');
	assert.equal(formatDurationLabel(18 * 60), '18 min');
	assert.equal(formatDurationLabel(3600), '1 hr');
	assert.equal(formatDurationLabel(3 * 3600 + 42 * 60), '3 hr 42 min');
});

test('formatDurationLabel localizes to Vietnamese', () => {
	assert.equal(formatDurationLabel(0, 'vi'), '');
	assert.equal(formatDurationLabel(NaN, 'vi'), '');
	assert.equal(formatDurationLabel(30, 'vi'), 'dưới một phút');
	assert.equal(formatDurationLabel(18 * 60, 'vi'), '18 phút');
	assert.equal(formatDurationLabel(3600, 'vi'), '1 tiếng');
	assert.equal(formatDurationLabel(3 * 3600 + 42 * 60, 'vi'), '3 tiếng 42 phút');
});

test('percentOf clamps and guards against a zero total', () => {
	assert.equal(percentOf(0, 100), 0);
	assert.equal(percentOf(50, 100), 50);
	assert.equal(percentOf(150, 100), 100);
	assert.equal(percentOf(-10, 100), 0);
	assert.equal(percentOf(10, 0), 0);
	assert.equal(percentOf(10, null), 0);
	assert.equal(percentOf(NaN, 100), 0);
});
