import { derived, writable } from "svelte/store";

const threshold = {
  lg: 1024,
  xl: 1280,
};
export const width = writable();

export const isLg = derived(width, ($width) => $width >= threshold.lg);
export const isXl = derived(width, ($width) => $width >= threshold.xl);
