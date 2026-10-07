import { test } from 'node:test';
import assert from 'node:assert/strict';
import { saveIndicator } from '../model/save-status.js';

const indicator = (over = {}) =>
	saveIndicator({ mode: 'edit', status: 'idle', isDirty: false, ...over });

test('a save in flight outranks the dirty flag', () => {
	// The old pill was `isDirty ? 'Unsaved' : 'Saved'`, so it read "Unsaved"
	// for the whole request — the save looked like it had not started. Work in
	// flight is what the user needs to know first.
	assert.equal(indicator({ status: 'saving', isDirty: true }).label, 'Saving…');
	assert.equal(indicator({ status: 'saving', isDirty: false }).label, 'Saving…');
	assert.equal(indicator({ status: 'saving' }).tone, 'busy');
});

test('a publish is in flight even when no save status was set', () => {
	// `publish()` never touches `ui.save.status`, so without this the pill
	// would sit on whatever the last save left behind.
	assert.equal(indicator({ status: 'saved', isPublishing: true }).label, 'Publishing…');
	assert.equal(indicator({ isPublishing: true, isDirty: true }).tone, 'busy');
});

test('a failure stays on the indicator until it is resolved', () => {
	assert.equal(indicator({ status: 'error' }).label, 'Save failed');
	assert.equal(indicator({ status: 'error' }).tone, 'error');
	assert.equal(indicator({ status: 'conflict' }).label, 'Conflict');
});

test('a clean edit entry reads Saved, a dirty one Unsaved', () => {
	assert.equal(indicator().label, 'Saved');
	assert.equal(indicator({ isDirty: true }).label, 'Unsaved');
	assert.equal(indicator({ isDirty: true }).tone, 'dirty');
});

test('a fresh create form claims nothing', () => {
	// Nothing has been saved yet, so "Saved" would be untrue. The pill only
	// appears once there is something to report.
	assert.equal(saveIndicator({ mode: 'create', status: 'idle', isDirty: false }), null);
	assert.equal(saveIndicator({ mode: 'create', status: 'idle', isDirty: true }).label, 'Unsaved');
	assert.equal(saveIndicator({ mode: 'create', status: 'saving', isDirty: true }).label, 'Saving…');
});
