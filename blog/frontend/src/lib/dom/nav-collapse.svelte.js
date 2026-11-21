import { browser } from '$app/environment';

const STORAGE_KEY = 'nav-compact';

/**
 * Manual collapse of the navigation rails. Below the `lg` breakpoint the
 * rails are compact (icon-only) or hidden by the responsive classes alone;
 * this store is the user's override at `lg` and up, where the rails are
 * normally full-width — collapsing them gives the main content the room.
 * Persisted per browser; both rails (public and dashboard) read the same
 * preference so the two never disagree.
 */
class NavCollapseStore {
	collapsed = $state(false);

	constructor() {
		if (browser) {
			try {
				this.collapsed = localStorage.getItem(STORAGE_KEY) === '1';
			} catch {
				// Storage can throw (private mode, quota); expanded is the default.
			}
		}
	}

	toggle() {
		this.collapsed = !this.collapsed;
		if (browser) {
			try {
				if (this.collapsed) localStorage.setItem(STORAGE_KEY, '1');
				else localStorage.removeItem(STORAGE_KEY);
			} catch {
				// The state still applies for this session.
			}
		}
	}
}

export const navCollapse = new NavCollapseStore();
