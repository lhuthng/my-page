// The editor's single feedback channel.
//
// Every message the editor emits goes through one of three methods, and the
// *trigger* decides which — not the severity.
//
//   live(key, value)        continuous state the editor already knows. Derived,
//                           never posted, and it goes away when the condition
//                           stops being true. No timer, no dismiss control.
//   banner(id, message, …)  something that blocks the user's intent and needs a
//                           decision. Keyed by id, so re-reporting the same
//                           failure refreshes the banner instead of stacking a
//                           duplicate. Always rendered with a `×`.
//   toast(message, …)       acknowledgement of an action. Auto-dismisses,
//                           pauses on hover, and carries a `×` for impatience.
//
// The rule that makes this work, and the reason it replaced `ui.notice` +
// `ui.noticeCritical`:
//
//   Every message must have at least one exit — time, resolution, or an
//   explicit dismiss. A message with none of the three may not ship.
//
// The old model had a single string slot and no lifetime rule, so a failed save
// left a red line in the toolbar for the rest of the session and upload
// progress outlived the upload. See docs/editor-feedback-ux.md.

import { MAX_TOASTS, TOAST_MS, overflowCount, remainingAfter } from '../model/feedback.js';

export { MAX_TOASTS, TOAST_MS };

export function createFeedback() {
	let toasts = $state([]);
	let banners = $state({});
	let liveSlots = $state({});

	// Timers and countdowns are machinery, not state — nothing renders them, so
	// they live in plain Maps instead of on the reactive objects. That also
	// keeps the toast objects safe to mutate freely.
	const timers = new Map();
	const remaining = new Map();
	const startedAt = new Map();

	let seq = 0;

	// ---- class 3: transient (toast) ----------------------------------------

	function arm(id, ms) {
		remaining.set(id, ms);
		startedAt.set(id, Date.now());
		timers.set(
			id,
			setTimeout(() => dropToast(id), ms)
		);
	}

	function dropToast(id) {
		const handle = timers.get(id);
		if (handle !== undefined) clearTimeout(handle);
		timers.delete(id);
		remaining.delete(id);
		startedAt.delete(id);
		toasts = toasts.filter((t) => t.id !== id);
	}

	function toast(message, { tone = 'success', ms = TOAST_MS } = {}) {
		const id = ++seq;
		toasts.push({ id, message, tone });
		// Bounded stack: the oldest is evicted, so a burst of saves can never
		// bury the editor under confirmations.
		for (let i = overflowCount(toasts.length); i > 0; i -= 1) {
			dropToast(toasts[0].id);
		}
		arm(id, ms);
		return id;
	}

	/** Hovering holds a toast open; leaving restarts whatever time was left. */
	function pauseToast(id) {
		const handle = timers.get(id);
		if (handle === undefined) return;
		clearTimeout(handle);
		timers.delete(id);
		remaining.set(
			id,
			remainingAfter(remaining.get(id) ?? 0, startedAt.get(id) ?? Date.now(), Date.now())
		);
	}

	function resumeToast(id) {
		if (timers.has(id) || !remaining.has(id)) return;
		startedAt.set(id, Date.now());
		timers.set(
			id,
			setTimeout(() => dropToast(id), remaining.get(id))
		);
	}

	// ---- class 2: sticky (banner) ------------------------------------------

	function banner(id, message, { tone = 'error', actions = [], onDismiss } = {}) {
		// Keyed by id: a retry that fails again refreshes this banner rather
		// than adding a second copy of the same complaint.
		banners[id] = { id, message, tone, actions, onDismiss };
	}

	function dismissBanner(id) {
		const existing = banners[id];
		if (!existing) return;
		delete banners[id];
		// `onDismiss` is how a banner's `×` doubles as a decision — discarding
		// a recovered local draft, for example. Called once, after removal.
		existing.onDismiss?.();
	}

	/** The `×`. Works for both dismissible classes. */
	function dismiss(id) {
		if (banners[id]) {
			dismissBanner(id);
			return;
		}
		dropToast(id);
	}

	// ---- class 1: live (inline status) -------------------------------------

	/**
	 * Set or clear a live slot. `null`/`undefined`/`''` clears it — this is the
	 * call that was missing before, and the reason upload progress used to
	 * outlive the upload.
	 */
	function live(key, value) {
		if (value === null || value === undefined || value === '') {
			delete liveSlots[key];
			return;
		}
		liveSlots[key] = value;
	}

	function destroy() {
		for (const handle of timers.values()) clearTimeout(handle);
		timers.clear();
		remaining.clear();
		startedAt.clear();
		toasts = [];
		banners = {};
		liveSlots = {};
	}

	return {
		get toasts() {
			return toasts;
		},
		get bannerList() {
			return Object.values(banners);
		},
		get liveSlots() {
			return liveSlots;
		},
		toast,
		pauseToast,
		resumeToast,
		banner,
		dismiss,
		live,
		destroy
	};
}
