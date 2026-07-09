import { createCommentNodeView } from '../model/view.js';

export function createThreadController({ commentsState, replyThreads }) {
	function ensureReplyThread(parentId, total = 0) {
		if (!replyThreads[parentId]) {
			replyThreads[parentId] = {
				expanded: false,
				fetching: false,
				endReached: true,
				lastId: 0,
				total,
				items: []
			};
		} else if (total != null && total >= 0) {
			replyThreads[parentId].total = total;
		}

		return replyThreads[parentId];
	}

	function mergeComments(existing, incoming) {
		const merged = new Map();
		[...existing, ...incoming].forEach((comment) => {
			merged.set(comment.id, createCommentNodeView(comment));
		});
		return [...merged.values()].sort((a, b) => b.id - a.id);
	}

	function updateRoots(page) {
		commentsState.roots = mergeComments(commentsState.roots, page.items);
		if (commentsState.roots.length > 0) {
			commentsState.lastId = commentsState.roots[commentsState.roots.length - 1].id;
		}
		commentsState.endReached = !page.hasMore;
		page.items.forEach((root) => {
			ensureReplyThread(root.id, root.direct_reply_count ?? 0);
		});
	}

	function updateReplies(parentId, page) {
		const thread = ensureReplyThread(parentId);
		thread.items = mergeComments(thread.items, page.items);
		if (thread.items.length > 0) {
			thread.lastId = thread.items[thread.items.length - 1].id;
		}
		thread.endReached = !page.hasMore;
		page.items.forEach((reply) => {
			ensureReplyThread(reply.id, reply.direct_reply_count ?? 0);
		});
		return thread;
	}

	function findCommentById(id) {
		if (id == null) return null;
		const root = commentsState.roots.find((item) => item.id === id);
		if (root) return root;

		for (const thread of Object.values(replyThreads)) {
			const found = thread.items.find((item) => item.id === id);
			if (found) return found;
		}
		return null;
	}

	async function expandReplyChain(commentId, loadReplies) {
		let currentId = commentId;
		while (currentId != null) {
			const current = findCommentById(currentId);
			if (!current) break;

			const thread = ensureReplyThread(currentId, current.direct_reply_count ?? 0);
			thread.expanded = true;
			if (thread.items.length === 0 && (current.direct_reply_count ?? 0) > 0) {
				thread.endReached = false;
				await loadReplies(currentId);
			}
			currentId = current.parent_id ?? null;
		}
	}

	function insertOptimisticComment(comment) {
		const normalized = createCommentNodeView(comment);
		if (normalized.parent_id == null) {
			updateRoots({ items: [normalized], hasMore: !commentsState.endReached });
			ensureReplyThread(normalized.id, 0);
			return;
		}

		const parent = findCommentById(normalized.parent_id);
		if (parent) {
			parent.direct_reply_count = (parent.direct_reply_count ?? 0) + 1;
		}

		const thread = ensureReplyThread(normalized.parent_id, parent?.direct_reply_count ?? 1);
		thread.total = Math.max(thread.total ?? 0, 1);
		updateReplies(normalized.parent_id, { items: [normalized], hasMore: true });
		thread.endReached = thread.items.length >= (thread.total ?? 0);
	}

	return {
		ensureReplyThread,
		updateRoots,
		updateReplies,
		findCommentById,
		expandReplyChain,
		insertOptimisticComment
	};
}
