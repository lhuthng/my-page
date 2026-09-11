import { test } from 'node:test';
import assert from 'node:assert/strict';
import { MAX_TOASTS, TOAST_MS, overflowCount, remainingAfter } from '../model/feedback.js';

test('the toast stack is bounded', () => {
	// A burst of saves must not be able to bury the editor, so the oldest is
	// evicted rather than the stack growing without limit.
	assert.equal(overflowCount(0), 0);
	assert.equal(overflowCount(MAX_TOASTS), 0);
	assert.equal(overflowCount(MAX_TOASTS + 1), 1);
	assert.equal(overflowCount(10), 7);
});

test('remainingAfter clamps at zero', () => {
	assert.equal(remainingAfter(TOAST_MS, 1000, 1000), TOAST_MS);
	assert.equal(remainingAfter(TOAST_MS, 1000, 2000), TOAST_MS - 1000);
	// A timer that fires late must not hand back a negative delay: setTimeout
	// would treat that as "fire now", which is right by accident rather than
	// by intent.
	assert.equal(remainingAfter(TOAST_MS, 1000, 99_000), 0);
});

test('a hover pause preserves what was left, not the full lifetime', () => {
	// Hovering 1.5s into a 4s toast leaves 2.5s, so hovering repeatedly cannot
	// keep a toast alive forever.
	const left = remainingAfter(TOAST_MS, 0, 1500);
	assert.equal(left, 2500);
	assert.equal(remainingAfter(left, 1500, 2000), 2000);
});

test('the toast lifetime matches the spec', () => {
	// docs/editor-feedback-ux.md §2: transient messages auto-dismiss at 4s.
	assert.equal(TOAST_MS, 4000);
	assert.equal(MAX_TOASTS, 3);
});
