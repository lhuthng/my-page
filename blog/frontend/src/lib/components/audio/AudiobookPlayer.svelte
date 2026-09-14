<script>
	import { onMount, untrack } from 'svelte';
	import { AudiobookPlayer } from '$lib/players/AudiobookPlayer.svelte.js';
	import { formatClock, percentOf } from '$lib/utils/duration.js';

	let {
		tracks = [],
		title = 'Audiobook',
		author = '',
		translator = '',
		coverUrl = null,
		storageKey = 'default'
	} = $props();

	// One engine per audiobook page, built from a deliberate one-time read of
	// the props: it owns the playlist for the lifetime of the page, so
	// re-creating it whenever a prop reference changed would reset playback.
	// (`untrack` states that intent rather than leaving it implicit.)
	const player = untrack(() => new AudiobookPlayer(tracks, { storageKey }));
	untrack(() => player.setMeta({ title, author, translator, coverUrl }));

	let audioEl = $state(null);
	/** Value shown while the listener drags the scrubber. */
	let scrub = $state(null);
	let playlistEl = $state(null);

	const current = $derived(player.current);
	const position = $derived(scrub ?? player.time);
	const duration = $derived(player.displayDuration);
	const playedPercent = $derived(percentOf(position, duration));
	const bufferedPercent = $derived(percentOf(player.buffered, duration));
	const isCurrent = (track) => track.id === current?.id;

	$effect(() => {
		if (!audioEl) return;
		player.attach(audioEl);
		return () => player.detach();
	});

	// Keep the playing chapter visible in a long playlist.
	$effect(() => {
		const index = player.index;
		if (!playlistEl) return;
		const active = playlistEl.querySelector(`[data-track-index="${index}"]`);
		active?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
	});

	onMount(() => {
		const onKeydown = (event) => {
			if (event.metaKey || event.ctrlKey || event.altKey) return;

			const target = event.target;
			if (
				target instanceof HTMLElement &&
				(target.isContentEditable ||
					['INPUT', 'TEXTAREA', 'SELECT', 'BUTTON'].includes(target.tagName))
			) {
				// Let form controls keep their own keyboard behaviour: the
				// scrubber, volume slider, and speed picker all use arrows and
				// space themselves.
				return;
			}

			switch (event.key) {
				case ' ':
				case 'k':
					event.preventDefault();
					player.toggle();
					break;
				case 'ArrowRight':
					event.preventDefault();
					player.skip(event.shiftKey ? player.skipSeconds : 5);
					break;
				case 'ArrowLeft':
					event.preventDefault();
					player.skip(event.shiftKey ? -player.skipSeconds : -5);
					break;
				case 'ArrowUp':
					event.preventDefault();
					player.setVolume(player.volume + 0.05);
					break;
				case 'ArrowDown':
					event.preventDefault();
					player.setVolume(player.volume - 0.05);
					break;
				case 'n':
					player.next();
					break;
				case 'p':
					player.previous();
					break;
				case 'm':
					player.toggleMute();
					break;
				default:
					break;
			}
		};

		window.addEventListener('keydown', onKeydown);
		return () => window.removeEventListener('keydown', onKeydown);
	});

	function commitSeek(event) {
		const ratio = Number(event.currentTarget.value) / 1000;
		player.seekToRatio(ratio);
		scrub = null;
	}
</script>

<section class="flex flex-col gap-4 bg-white rounded-xl p-4 text-dark">
	<audio bind:this={audioEl} preload="metadata" class="hidden"></audio>

	{#if tracks.length === 0}
		<p class="py-8 text-center text-dark/50">This audiobook has no tracks yet.</p>
	{:else}
		<!-- Header: cover, book and chapter identity -->
		<div class="flex gap-4 items-start">
			{#if coverUrl}
				<img src={coverUrl} alt={`Cover of ${title}`} class="w-24 h-24 rounded-xl object-cover" />
			{:else}
				<div class="w-24 h-24 rounded-xl bg-primary/20 shrink-0"></div>
			{/if}

			<div class="flex flex-col min-w-0 gap-1">
				<h1 class="text-xl font-semibold line-clamp-2">{title}</h1>
				{#if translator}
					<p class="text-sm text-dark/60">Translated by {translator}</p>
				{:else if author}
					<p class="text-sm text-dark/60">{author}</p>
				{/if}
				<p class="text-sm font-medium line-clamp-1">
					<span class="text-dark/50">Chapter {current?.number ?? 1} of {tracks.length}:</span>
					{current?.title ?? ''}
				</p>
			</div>
		</div>

		<!-- Resume offer -->
		{#if player.resumeOffer}
			<div
				class="flex items-center gap-3 bg-accent-yellow-light-4 border border-accent-yellow rounded-lg p-3"
			>
				<span class="text-sm grow">
					Resume from {formatClock(player.resumeOffer.time)}?
				</span>
				<button
					class="rounded-full bg-dark text-white text-sm px-3 py-1 hover:bg-dark/90"
					onclick={() => player.acceptResume()}
				>
					Resume
				</button>
				<button
					class="rounded-full border border-dark/30 text-sm px-3 py-1 hover:bg-dark/10"
					onclick={() => player.dismissResume()}
				>
					Start over
				</button>
			</div>
		{/if}

		{#if player.interrupted}
			<p class="text-sm text-accent-red">
				Playback was blocked by the browser. Press play to start listening.
			</p>
		{/if}

		<!-- Scrubber -->
		<div class="flex flex-col gap-1">
			<div class="relative h-6 flex items-center">
				<div class="absolute inset-x-0 h-2 rounded-full bg-dark/15 overflow-hidden">
					<div class="h-full bg-dark/25" style="width: {bufferedPercent}%"></div>
				</div>
				<div
					class="absolute left-0 h-2 rounded-full bg-primary pointer-events-none"
					style="width: {playedPercent}%"
				></div>
				<div
					class="absolute w-4 h-4 rounded-full bg-primary border-2 border-white shadow pointer-events-none"
					style="left: calc({playedPercent}% - 8px)"
				></div>
				<input
					type="range"
					min="0"
					max="1000"
					step="1"
					class="absolute inset-0 w-full opacity-0 cursor-pointer"
					aria-label="Seek within chapter"
					value={Math.round(playedPercent * 10)}
					oninput={(event) => (scrub = (Number(event.currentTarget.value) / 1000) * duration)}
					onchange={commitSeek}
				/>
			</div>

			<div class="flex justify-between text-xs text-dark/60 tabular-nums">
				<span>{formatClock(position)}</span>
				<span>{formatClock(duration)}</span>
			</div>
		</div>

		<!-- Transport -->
		<div class="flex items-center justify-center gap-2 flex-wrap">
			<button
				class="rounded-full p-2 hover:bg-dark/10 disabled:opacity-30"
				disabled={!player.hasPrevious}
				onclick={() => player.previous()}
				aria-label="Previous chapter"
				title="Previous chapter (p)"
			>
				<svg class="w-6 h-6 fill-dark" viewBox="0 0 24 24">
					<path d="M7 6h2v12H7zm3 6l9 6V6z" />
				</svg>
			</button>

			<button
				class="relative rounded-full p-2 hover:bg-dark/10"
				onclick={() => player.skip(-player.skipSeconds)}
				aria-label="Skip back {player.skipSeconds} seconds"
				title="Skip back (Shift+←)"
			>
				<svg class="w-7 h-7 fill-dark" viewBox="0 0 24 24">
					<path d="M12 5V2L7 6l5 4V7a5 5 0 1 1-5 5H5a7 7 0 1 0 7-7z" />
				</svg>
				<span class="absolute inset-0 flex items-center justify-center text-[9px] font-bold pt-0.5">
					{player.skipSeconds}
				</span>
			</button>

			<button
				class="rounded-full bg-dark text-white p-3 hover:bg-dark/90"
				onclick={() => player.toggle()}
				aria-label={player.playing ? 'Pause' : 'Play'}
				title="Play or pause (Space)"
			>
				{#if player.playing}
					<svg class="w-7 h-7 fill-white" viewBox="0 0 24 24">
						<path d="M7 5h4v14H7zm6 0h4v14h-4z" />
					</svg>
				{:else}
					<svg class="w-7 h-7 fill-white" viewBox="0 0 24 24">
						<path d="M8 5l11 7-11 7z" />
					</svg>
				{/if}
			</button>

			<button
				class="relative rounded-full p-2 hover:bg-dark/10"
				onclick={() => player.skip(player.skipSeconds)}
				aria-label="Skip forward {player.skipSeconds} seconds"
				title="Skip forward (Shift+→)"
			>
				<svg class="w-7 h-7 fill-dark" viewBox="0 0 24 24">
					<path d="M12 5V2l5 4-5 4V7a5 5 0 1 0 5 5h2a7 7 0 1 1-7-7z" />
				</svg>
				<span class="absolute inset-0 flex items-center justify-center text-[9px] font-bold pt-0.5">
					{player.skipSeconds}
				</span>
			</button>

			<button
				class="rounded-full p-2 hover:bg-dark/10 disabled:opacity-30"
				disabled={!player.hasNext}
				onclick={() => player.next()}
				aria-label="Next chapter"
				title="Next chapter (n)"
			>
				<svg class="w-6 h-6 fill-dark" viewBox="0 0 24 24">
					<path d="M15 6h2v12h-2zM5 6l9 6-9 6z" />
				</svg>
			</button>
		</div>

		<!-- Secondary controls -->
		<div class="flex items-center gap-3 flex-wrap justify-center text-sm">
			<div class="flex items-center gap-2">
				<button
					class="rounded-full p-1.5 hover:bg-dark/10"
					onclick={() => player.toggleMute()}
					aria-label={player.muted ? 'Unmute' : 'Mute'}
				>
					<svg class="w-5 h-5 fill-dark" viewBox="0 0 24 24">
						{#if player.muted || player.volume === 0}
							<path
								d="M4 9v6h4l5 4V5L8 9H4zm12.5 3l2.5 2.5 1-1L17.5 11l2.5-2.5-1-1L16.5 10 14 7.5l-1 1L15.5 11 13 13.5l1 1z"
							/>
						{:else}
							<path d="M4 9v6h4l5 4V5L8 9H4zm12 3a4 4 0 0 0-2-3.46v6.92A4 4 0 0 0 16 12z" />
						{/if}
					</svg>
				</button>
				<input
					type="range"
					min="0"
					max="1"
					step="0.05"
					class="w-24 accent-primary"
					aria-label="Volume"
					value={player.volume}
					oninput={(event) => player.setVolume(Number(event.currentTarget.value))}
				/>
			</div>

			<details class="relative">
				<summary
					class="list-none cursor-pointer rounded-full border border-dark/20 px-3 py-1 hover:bg-dark/10"
				>
					{player.rate}×
				</summary>
				<ul
					class="absolute bottom-full mb-1 z-10 bg-white border border-dark/20 rounded-lg shadow-lg py-1 min-w-20"
				>
					{#each player.playbackRates as value}
						<li>
							<button
								class="w-full text-left px-3 py-1 hover:bg-dark/10 {value === player.rate
									? 'font-semibold'
									: ''}"
								onclick={(event) => {
									player.setRate(value);
									event.currentTarget.closest('details')?.removeAttribute('open');
								}}
							>
								{value}×
							</button>
						</li>
					{/each}
				</ul>
			</details>

			<details class="relative">
				<summary
					class="list-none cursor-pointer rounded-full border border-dark/20 px-3 py-1 hover:bg-dark/10"
				>
					{#if player.sleepMode === 'chapter'}
						Sleep: chapter
					{:else if player.sleepMode}
						Sleep: {formatClock(player.sleepRemaining)}
					{:else}
						Sleep timer
					{/if}
				</summary>
				<ul
					class="absolute bottom-full mb-1 z-10 bg-white border border-dark/20 rounded-lg shadow-lg py-1 min-w-32"
				>
					{#each player.sleepPresets as minutes}
						<li>
							<button
								class="w-full text-left px-3 py-1 hover:bg-dark/10"
								onclick={(event) => {
									player.startSleep(minutes);
									event.currentTarget.closest('details')?.removeAttribute('open');
								}}
							>
								{minutes} minutes
							</button>
						</li>
					{/each}
					<li>
						<button
							class="w-full text-left px-3 py-1 hover:bg-dark/10"
							onclick={(event) => {
								player.startSleep('chapter');
								event.currentTarget.closest('details')?.removeAttribute('open');
							}}
						>
							End of chapter
						</button>
					</li>
					{#if player.sleepMode}
						<li>
							<button
								class="w-full text-left px-3 py-1 text-accent-red hover:bg-dark/10"
								onclick={(event) => {
									player.cancelSleep();
									event.currentTarget.closest('details')?.removeAttribute('open');
								}}
							>
								Cancel timer
							</button>
						</li>
					{/if}
				</ul>
			</details>

			<label class="flex items-center gap-1 text-dark/60">
				Skip
				<select
					class="rounded border border-dark/20 bg-white px-1 py-0.5"
					aria-label="Skip interval in seconds"
					value={player.skipSeconds}
					onchange={(event) => player.setSkipSeconds(Number(event.currentTarget.value))}
				>
					{#each [10, 15, 30, 45] as seconds}
						<option value={seconds}>{seconds}s</option>
					{/each}
				</select>
			</label>

			<button class="text-dark/60 hover:underline" onclick={() => player.clearSaved()}>
				Restart
			</button>
		</div>

		<!-- Playlist -->
		<div class="flex flex-col gap-2">
			<div class="flex items-baseline justify-between">
				<h2 class="font-semibold">Chapters</h2>
				<span class="text-xs text-dark/50">{tracks.length} tracks</span>
			</div>

			<ol
				bind:this={playlistEl}
				class="flex flex-col max-h-96 overflow-y-auto custom-scrollbar divide-y divide-background"
			>
				{#each tracks as track, index (track.id)}
					{@const active = isCurrent(track)}
					<li data-track-index={index}>
						<button
							class="w-full flex items-center gap-3 py-2 px-2 text-left rounded-lg {active
								? 'bg-primary/20'
								: 'hover:bg-dark/5'}"
							onclick={() => player.load(index, { play: true })}
							aria-current={active ? 'true' : undefined}
						>
							<span class="w-8 text-right text-sm shrink-0 {active ? 'text-dark' : 'text-dark/40'}">
								{#if active && player.playing}
									<svg class="w-4 h-4 inline fill-primary" viewBox="0 0 24 24">
										<path d="M7 5h3v14H7zm7 0h3v14h-3z" />
									</svg>
								{:else}
									{track.number}
								{/if}
							</span>

							<span class="grow min-w-0 flex flex-col">
								<span class="text-sm font-medium line-clamp-1 {active ? '' : 'text-dark/80'}">
									{track.title}
								</span>
								{#if player.failures[track.id]}
									<span class="text-xs text-accent-red">{player.failures[track.id]}</span>
								{/if}
							</span>

							<span class="text-xs text-dark/50 tabular-nums shrink-0">
								{formatClock(track.duration_seconds)}
							</span>
						</button>
					</li>
				{/each}
			</ol>
		</div>

		<p class="sr-only" aria-live="polite">
			{player.playing ? 'Playing' : 'Paused'}
			{current?.title ?? ''}
		</p>
	{/if}
</section>
