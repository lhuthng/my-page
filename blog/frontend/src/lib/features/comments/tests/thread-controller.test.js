import test from 'node:test';
import assert from 'node:assert/strict';
import { createThreadController } from '../controllers/thread-controller.js';

test('thread controller merges and dedupes roots', () => {
	const commentsState = { roots: [], lastId: 0, endReached: false };
	const replyThreads = {};
	const controller = createThreadController({ commentsState, replyThreads });

	controller.updateRoots({
		items: [{ id: 2 }, { id: 1 }],
		hasMore: true
	});
	controller.updateRoots({
		items: [{ id: 3 }, { id: 2 }],
		hasMore: false
	});

	assert.deepEqual(
		commentsState.roots.map((item) => item.id),
		[3, 2, 1]
	);
	assert.equal(commentsState.lastId, 1);
	assert.equal(commentsState.endReached, true);
});

test('thread controller inserts optimistic replies and bumps counts', () => {
	const commentsState = {
		roots: [{ id: 10, direct_reply_count: 0 }],
		lastId: 10,
		endReached: false
	};
	const replyThreads = {};
	const controller = createThreadController({ commentsState, replyThreads });

	controller.ensureReplyThread(10, 0);
	controller.insertOptimisticComment({
		id: 20,
		parent_id: 10,
		direct_reply_count: 0
	});

	assert.equal(commentsState.roots[0].direct_reply_count, 1);
	assert.deepEqual(
		replyThreads[10].items.map((item) => item.id),
		[20]
	);
});
