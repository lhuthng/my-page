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
		// attach() reads and writes engine state (restore() loads the saved
		// track), so it must not be tracked: tracking would re-run this effect
		// on every playlist/index change, and the re-attach's audio.load()
		// aborts any play() the user just triggered (AbortError) — the player
		// then shows "blocked by browser" instead of playing.
		untrack(() => player.attach(audioEl));
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

	// The invisible range input maps pointer positions across
	// (width - native thumb width), so clicks near the edges land off from
	// the cursor. Pointer coordinates on the bar itself map exactly.
	function ratioFromPointer(event, el) {
		const rect = el.getBoundingClientRect();
		return Math.min(1, Math.max(0, (event.clientX - rect.left) / rect.width));
	}

	function previewScrub(event) {
		scrub = ratioFromPointer(event, event.currentTarget) * duration;
	}

	function commitPointerSeek(event) {
		player.seekToRatio(ratioFromPointer(event, event.currentTarget));
		scrub = null;
	}
</script>

<section class="flex flex-col gap-4 text-dark border-t-2 border-dark/10 pt-4">
	<audio bind:this={audioEl} preload="metadata" class="hidden"></audio>

	{#if tracks.length === 0}
		<p class="py-8 text-center text-base text-dark/50">This audiobook has no tracks yet.</p>
	{:else}
		<!-- Chapter identity -->
		<p class="text-lg md:text-xl font-bold line-clamp-2">
			Chapter {current?.number ?? 1} - {current?.title ?? ''}
		</p>

		<!-- Resume offer -->
		{#if player.resumeOffer}
			<div
				class="flex items-center gap-3 bg-accent-yellow-light-4 border border-accent-yellow rounded-lg p-3"
			>
				<span class="text-base grow">
					Resume from {formatClock(player.resumeOffer.time)}?
				</span>
				<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="green">
					<button onclick={() => player.acceptResume()}>Resume</button>
				</div>
				<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="blue">
					<button onclick={() => player.dismissResume()}>Start over</button>
				</div>
			</div>
		{/if}

		{#if player.interrupted}
			<p class="text-base text-accent-red">
				Playback was blocked by the browser. Press play to start listening.
			</p>
		{/if}

		<!-- Progress -->
		<div class="flex items-center gap-3">
			<span class="text-base tabular-nums shrink-0">{formatClock(position)}</span>
			<div
				class="relative h-6 flex items-center grow cursor-pointer touch-none"
				onpointerdown={(event) => {
					event.currentTarget.setPointerCapture(event.pointerId);
					previewScrub(event);
				}}
				onpointermove={(event) => {
					if (event.buttons > 0) previewScrub(event);
				}}
				onpointerup={commitPointerSeek}
				onpointercancel={() => (scrub = null)}
			>
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
					class="absolute inset-0 w-full opacity-0 pointer-events-none"
					aria-label="Seek within chapter"
					value={Math.round(playedPercent * 10)}
					oninput={(event) => (scrub = (Number(event.currentTarget.value) / 1000) * duration)}
					onchange={commitSeek}
				/>
			</div>
			<span class="text-base tabular-nums shrink-0">{formatClock(duration)}</span>
		</div>

		<!-- Transport -->
		<div class="flex items-center justify-center gap-4">
			<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="blue">
				<button
					class="p-2!"
					disabled={!player.hasPrevious}
					onclick={() => player.previous()}
					aria-label="Previous chapter"
					title="Previous chapter (p)"
				>
					<svg class="w-6 h-6 fill-white" viewBox="0 0 24 24">
						<path d="M7 6h2v12H7zm3 6l9 6V6z" />
					</svg>
				</button>
			</div>

			<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="green">
				<button
					class="p-3!"
					onclick={() => player.toggle()}
					aria-label={player.playing ? 'Pause' : 'Play'}
					title="Play or pause (Space)"
				>
					{#if player.playing}
						<svg class="w-8 h-8 fill-white" viewBox="0 0 24 24">
							<path d="M7 5h4v14H7zm6 0h4v14h-4z" />
						</svg>
					{:else}
						<svg class="w-8 h-8 fill-white" viewBox="0 0 24 24">
							<path d="M8 5l11 7-11 7z" />
						</svg>
					{/if}
				</button>
			</div>

			<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="blue">
				<button
					class="p-2!"
					disabled={!player.hasNext}
					onclick={() => player.next()}
					aria-label="Next chapter"
					title="Next chapter (n)"
				>
					<svg class="w-6 h-6 fill-white" viewBox="0 0 24 24">
						<path d="M15 6h2v12h-2zM5 6l9 6-9 6z" />
					</svg>
				</button>
			</div>
		</div>

		<!-- Secondary controls -->
		<div class="flex items-center gap-3 flex-wrap justify-center text-base">
			<div class="flex items-center gap-2">
				<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="white">
					<button
						class="p-1.5!"
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
				</div>
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

			<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="white">
				<details class="relative">
					<summary class="list-none cursor-pointer">
						{player.rate}×
					</summary>
					<ul
						class="absolute bottom-full left-0 z-30 mb-2 overflow-hidden rounded-lg border border-dark/20 bg-white py-1 shadow-lg min-w-20"
					>
						{#each player.playbackRates as value}
							<li>
								<button
									class="w-full text-left text-base px-3 py-1 hover:bg-dark/10 {value ===
									player.rate
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
			</div>

			<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="white">
				<details class="relative">
					<summary class="list-none cursor-pointer">
						{#if player.sleepMode === 'chapter'}
							Sleep: chapter
						{:else if player.sleepMode}
							Sleep: {formatClock(player.sleepRemaining)}
						{:else}
							Sleep timer
						{/if}
					</summary>
					<ul
						class="absolute bottom-full left-0 z-30 mb-2 overflow-hidden rounded-lg border border-dark/20 bg-white py-1 shadow-lg min-w-32"
					>
						{#each player.sleepPresets as minutes}
							<li>
								<button
									class="w-full text-left text-base px-3 py-1 hover:bg-dark/10"
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
								class="w-full text-left text-base px-3 py-1 hover:bg-dark/10"
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
									class="w-full text-left text-base px-3 py-1 text-accent-red hover:bg-dark/10"
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
			</div>

			<label class="flex items-center gap-1 text-base text-dark/60">
				Skip
				<select
					class="rounded border border-dark/20 bg-white px-1 py-0.5 text-base"
					aria-label="Skip interval in seconds"
					value={player.skipSeconds}
					onchange={(event) => player.setSkipSeconds(Number(event.currentTarget.value))}
				>
					{#each [10, 15, 30, 45] as seconds}
						<option value={seconds}>{seconds}s</option>
					{/each}
				</select>
			</label>

			<div class="duo-btn w-fit" data-duo-shape="round" data-duo-color="white">
				<button onclick={() => player.clearSaved()}>Restart</button>
			</div>
		</div>

		<!-- Playlist -->
		<div class="flex flex-col gap-2">
			<div class="flex items-baseline justify-between">
				<h2 class="text-lg font-semibold">Chapters</h2>
				<span class="text-base text-dark/50">{tracks.length} tracks</span>
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
							<span class="grow min-w-0 flex flex-col">
								<span
									class="font-['Baloo_2',Roboto,sans-serif] text-base font-medium line-clamp-1 {active
										? ''
										: 'text-dark/80'}"
								>
									Chapter {track.number} - {track.title}
								</span>
								{#if player.failures[track.id]}
									<span class="text-base text-accent-red">{player.failures[track.id]}</span>
								{/if}
							</span>

							<span class="text-base text-dark/50 tabular-nums shrink-0">
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
