// Pure helpers for the editor feedback store.
//
// Kept separate from `feedback.svelte.js` on purpose: runes only exist once
// Svelte has compiled a `.svelte.js` module, so anything holding `$state` is
// unreachable from the plain-Node test runner. The arithmetic and the stack
// policy live here where they can be tested.

export const TOAST_MS = 4000;
export const MAX_TOASTS = 3;

/**
 * How many of the oldest toasts must be dropped to get back inside the cap.
 *
 * A burst of saves must not be able to bury the editor under toasts, so the
 * stack is bounded rather than unbounded.
 */
export function overflowCount(length, max = MAX_TOASTS) {
	return Math.max(0, length - max);
}

/**
 * Time left on a toast armed with `remainingMs` at `startedAt`.
 *
 * Used when a hover pauses a toast and the pointer leaves again. Clamped at
 * zero so a timer that fires late can never hand back a negative delay, which
 * `setTimeout` would treat as "fire now" — the right outcome, but by accident.
 */
export function remainingAfter(remainingMs, startedAt, now) {
	return Math.max(0, remainingMs - (now - startedAt));
}
