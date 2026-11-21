import { browser } from '$app/environment';
import { SvelteMap } from 'svelte/reactivity';

const STORAGE_KEY = 'nav-compact';
const DECLINED_PREFIX = 'nav-compact-declined:';

/**
 * Collapse of the navigation rails, with two layers:
 *
 * - `userCollapsed` — the manual toggle, persisted per browser. Below the `lg`
 *   breakpoint the rails are compact (icon-only) or hidden by the responsive
 *   classes alone; this override matters at `lg` and up, where the rails are
 *   normally full-width.
 * - **Requests** — pages that need more room (a fixed-width game frame that
 *   overflows with the rail expanded) claim a compact rail for as long as
 *   they are mounted. Requests are transient: releasing them (navigating
 *   away) returns the rail to the user's preference.
 *
 * `collapsed` is the effective state both rails render. The user always wins:
 * expanding the rail while a request is held declines that request (remembered
 * per source for the session, so a page does not re-claim after its human said
 * no), and a manual collapse stands even when nothing requests it.
 */
class NavCollapseStore {
	userCollapsed = $state(false);
	// SvelteMap, not $state(new Map()): the derived `collapsed` reads `.size`,
	// and only a SvelteMap's mutations are tracked there.
	/** @type {SvelteMap<string, { onDecline?: () => void }>} */
	requests = new SvelteMap();

	collapsed = $derived(this.userCollapsed || this.requests.size > 0);

	constructor() {
		if (browser) {
			try {
				this.userCollapsed = localStorage.getItem(STORAGE_KEY) === '1';
			} catch {
				// Storage can throw (private mode, quota); expanded is the default.
			}
		}
	}

	/**
	 * The manual toggle. When pages hold requests, expanding is a decline: the
	 * requests are dropped (and remember the refusal), so the rail does not
	 * snap back behind the user's back.
	 */
	toggle() {
		if (this.requests.size > 0 && this.collapsed) {
			for (const request of [...this.requests.values()]) request.onDecline?.();
			this.requests.clear();
		}
		this.userCollapsed = !this.userCollapsed;
		if (browser) {
			try {
				if (this.userCollapsed) localStorage.setItem(STORAGE_KEY, '1');
				else localStorage.removeItem(STORAGE_KEY);
			} catch {
				// The state still applies for this session.
			}
		}
	}

	/** A page claims the compact rail while it is mounted. */
	request(id, onDecline) {
		if (!this.requests.has(id)) this.requests.set(id, { onDecline });
	}

	release(id) {
		this.requests.delete(id);
	}

	/** Whether the source already said no this session. */
	isDeclined(id) {
		if (!browser) return false;
		try {
			return sessionStorage.getItem(DECLINED_PREFIX + id) === '1';
		} catch {
			return false;
		}
	}

	/** Record a refusal so the same source stops asking until the tab closes. */
	decline(id) {
		if (!browser) return;
		try {
			sessionStorage.setItem(DECLINED_PREFIX + id, '1');
		} catch {
			// Without storage the page simply asks again next visit.
		}
	}
}

export const navCollapse = new NavCollapseStore();
