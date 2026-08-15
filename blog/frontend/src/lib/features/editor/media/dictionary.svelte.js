/**
 * Owns the editor's media lookup: media the user just dropped in but hasn't
 * saved yet ("offline"), and media the body references that already exists on
 * the server ("online", fetched by short name as they're typed).
 *
 * This used to be `MediaDictionaryController.svelte`'s own component state,
 * exposed to its parent by handing seven functions *up* into `$state` slots
 * the parent declared for that purpose (`PostEditor.svelte`'s
 * `searchMedia`/`isOnline`/`isOffline`/`getNewMedia`/`clearNewMedia`, filled in
 * via `untrack(() => registerX)(...)` calls during the child's own setup).
 * That's state mutation during render, working in a direction the framework
 * can't help reason about.
 *
 * Now the view-model creates one of these directly — no functions cross a
 * component boundary in either direction. `MediaDictionaryController`
 * receives the finished object as a prop and only renders it.
 */
export function createMediaDictionary({ fetchImpl = fetch } = {}) {
	/** short_name -> {file, url, type} for media dropped but not yet uploaded */
	let newMedia = $state({});
	/** short_name -> url | null (pending) | undefined (missing), for existing media */
	let onlineMedia = $state({});

	const offlineMedia = $derived(
		Object.fromEntries(
			Object.entries(newMedia).map(([key, value]) => [key, { url: value.url, type: value.type }])
		)
	);

	const dictionary = $derived({
		...Object.fromEntries(Object.entries(offlineMedia).map(([key, value]) => [key, value.url])),
		...Object.fromEntries(Object.entries(onlineMedia).filter(([, v]) => v !== undefined))
	});

	async function searchOnlineRaw(keys) {
		const missingKeys = keys.filter((key) => !(key in onlineMedia));
		if (missingKeys.length === 0) return;

		missingKeys.forEach((key) => (onlineMedia[key] = null));

		await Promise.all(
			missingKeys.map(async (key) => {
				try {
					const res = await fetchImpl('/api/media/s/' + key, { method: 'GET' });
					if (res.ok) {
						onlineMedia[key] = (await res.json()).url;
					} else {
						onlineMedia[key] = undefined;
						setTimeout(() => delete onlineMedia[key], 5000);
					}
				} catch {
					onlineMedia[key] = undefined;
					setTimeout(() => delete onlineMedia[key], 5000);
				}
			})
		);
	}

	return {
		get dictionary() {
			return dictionary;
		},
		get offline() {
			return offlineMedia;
		},
		get online() {
			return onlineMedia;
		},

		isOnline: (keyword) => keyword in onlineMedia && onlineMedia[keyword] != null,
		isOffline: (keyword) => keyword in offlineMedia && offlineMedia[keyword] != null,
		getNew: (keyword) => newMedia[keyword],

		/**
		 * Look up a batch of keys. The content editor already debounces how
		 * often this is called (it only fires on its own render-debounce tick),
		 * so there is no separate debounce layer here — and each key is only
		 * ever fetched once regardless of how often it's requested.
		 */
		search: searchOnlineRaw,

		uploadNew(media) {
			const names = [];
			media.forEach((medium) => {
				newMedia[medium.name] = medium;
				names.push(medium.name);
			});
			searchOnlineRaw(names);
		},

		clearNew(names) {
			names.forEach((name) => {
				// The name has just been confirmed uploaded to the server, so the
				// local blob preview is no longer needed — unlike the cover
				// uploaders, nothing outside this dictionary keeps a reference to
				// it (the dictionary's own `dictionary` getter switches that key
				// over to the online URL in the same tick).
				if (newMedia[name]?.url) URL.revokeObjectURL(newMedia[name].url);
				delete newMedia[name];
			});
			searchOnlineRaw(names);
		},

		changeName(oldName, newName) {
			if (newName in newMedia) return false;
			newMedia[newName] = newMedia[oldName];
			delete newMedia[oldName];
			if (!(newName in onlineMedia)) searchOnlineRaw([newName]);
			return true;
		},

		/** Revoke every still-pending offline preview, e.g. on editor teardown. */
		destroy() {
			Object.values(newMedia).forEach((medium) => {
				if (medium?.url) URL.revokeObjectURL(medium.url);
			});
		}
	};
}
