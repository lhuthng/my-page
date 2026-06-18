const threshold = {
	lg: 1024,
	xl: 1280
};

class WindowStore {
	width = $state(0);
	isLg = $derived(this.width >= threshold.lg);
	isXl = $derived(this.width >= threshold.xl);
}

export const win = new WindowStore();
