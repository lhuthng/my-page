// Pure helpers for the editor feedback store.
//
// Kept separate from `feedback.svelte.js` on purpose: runes only exist once
// Svelte has compiled a `.svelte.js` module, so anything holding `$state` is
// unreachable from the plain-Node test runner. The arithmetic and the stack
// policy live here where they can be tested.

export const TOAST_MS = 4000;
export const MAX_TOASTS = 3;

/**
 * How long a message that is *not* a plain success stays up.
 *
 * A success is an acknowledgement: you already know what you did, and the
 * confirmation just closes the loop. Anything else — "Nothing to save.", a
 * neutral report you have to actually read — is information, and 4 s is long
 * enough to miss. Failures are stickier still (banners, no timer at all), so
 * this only ever applies to the transient class.
 */
export const ATTENTION_MS = 7000;

/** The default lifetime for a transient message, by tone. */
export function toastMsFor(tone) {
	return tone === 'success' ? TOAST_MS : ATTENTION_MS;
}

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
