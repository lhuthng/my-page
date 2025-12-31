import { writable } from "svelte/store";

export function pick(array) {
  return array[(Math.random() * array.length) >> 0];
}

export const pbody = writable(undefined);
