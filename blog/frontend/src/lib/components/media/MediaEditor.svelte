<script>
	import { cache } from '$lib/utils/cache.svelte.js';
	import { auth } from '$lib/auth/user.svelte.js';
	import { useDebounce } from '$lib/utils/debounce';
	import { untrack } from 'svelte';

	import MediaDirectory from './MediaDirectory.svelte';
	import MediaEditForm from './MediaEditForm.svelte';
	import MediaUploadPreview from './MediaUploadPreview.svelte';
	import Portal from '$lib/components/shell/Portal.svelte';
	import { formatDateOnly, formatDateTime } from '$lib/utils/datetime.js';

	let { keyword, detailPanel } = $props();

	let requestCache = $state({});
	let selection = $state();

	// Debounce the search keywords
	let deKeyword = $state(untrack(() => keyword));
	let debounce = useDebounce(async (searchKeywords) => {
		deKeyword = searchKeywords;
		selection = undefined;
	}, 300);

	// Check for results
	async function search(keyword) {
		// An empty box asks for nothing in particular, and the backend answers
		// that with the most recently uploaded files — the query already orders
		// by upload date, so this is "show me what I last put in" rather than an
		// unbounded listing. One character is skipped: it is noise to search on,
		// and typing it is not a request to browse.
		if (keyword.length === 1) return;
		if (requestCache[keyword] === undefined) {
			const req = { status: 'waiting' };
			requestCache[keyword] = { ...req };

			// `/api/media/all`, not `/api/media`. The backend nests media under
			// `/media/all`, `/media/d/:name`, `/media/i/:name` — there is no bare
			// `/media` route, and a request for one fell through to the static-file
			// service and failed, so this panel never listed anything.
			//
			// The `all` route also earns its keep beyond the correct path: its
			// server handler re-roots every result's `url` through
			// `fixClientRoute`, and the generic catch-all proxy does not. The
			// backend answers with a backend-relative `media/i/<name>`, which a
			// tile would resolve against the current route and request from the
			// wrong place. Search is the endpoint that makes the thumbnails
			// loadable, not just the one that answers.
			const res = await fetch(`/api/media/all?term=${encodeURIComponent(keyword)}&size=24`, {
				method: 'GET',
				headers: { Authorization: auth() }
			});

			if (res.ok) {
				req.status = 'success';
				req.results = (await res.json()).results;
			} else {
				req.status = 'failed';
			}
			requestCache[keyword] = { ...req };
		}
	}

	$effect(() => {
		debounce.update(keyword);
		return () => debounce.destroy();
	});

	$effect(async () => {
		search(deKeyword);
	});
</script>

<div class="relative full">
	<MediaDirectory class="full p-2" cellWidth="120px" cellHeight="200px" onclick={() => {}}>
		{#if requestCache[deKeyword]?.status === 'success'}
			{#each requestCache[deKeyword]?.results as item, index (item.short_name)}
				<!-- The upload date is rendered in the reader's own zone, matching the
				     details panel: both describe a moment in their own history, so a
				     tile and its panel must not disagree about which day it was. -->
				<MediaUploadPreview
					size={80}
					file={{ name: item.short_name, url: item.url }}
					meta={formatDateOnly(item.created_at, { timeZone: undefined })}
					metaTitle={formatDateTime(item.created_at)}
					isSelected={selection === item.short_name}
					onclick={async () => {
						if (!cache.details[item.short_name]) {
							const details = { status: 'waiting' };
							cache.details[item.short_name] = { ...details };
							const res = await fetch(`/api/media/d/${item.short_name}`, {
								method: 'GET',
								headers: {
									Authorization: auth()
								}
							});
							if (res.ok) {
								details.status = 'success';
								details.result = await res.json();
							} else {
								details.status = 'failed';
							}
							cache.details[item.short_name] = { ...details };
						}
						selection = item.short_name;
					}}
					ondblclick={() => {}}
				/>
			{/each}
		{:else}
			<div
				class="col-span-full flex flex-col items-center justify-center gap-2 py-12 text-dark/40 text-center px-4"
			>
				<p class="text-lg">Search media by keyword</p>
				<p class="text-sm text-dark/30">
					Your most recent uploads are shown already. Type 2 or more characters to narrow them down,
					then select a tile to edit its details.
				</p>
			</div>
		{/if}
	</MediaDirectory>
</div>
<Portal class="p-2" target={detailPanel}>
	<MediaEditForm
		shortName={selection}
		onShortNameChanged={(newShortName) => {
			delete requestCache[deKeyword];
			search(deKeyword);
			selection = undefined;
		}}
	/>
</Portal>
