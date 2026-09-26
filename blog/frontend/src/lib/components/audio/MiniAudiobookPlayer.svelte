<script>
	import { fly } from 'svelte/transition';
	import { audiobookSession } from '$lib/players/AudiobookSession.svelte.js';
	import { formatClock, percentOf } from '$lib/utils/duration.js';

	const session = audiobookSession;

	const engine = $derived(session.current?.engine ?? null);
	const book = $derived(session.current?.book ?? null);
	const track = $derived(engine?.current ?? null);
	const href = $derived(book?.slug ? `/audiobooks/${book.slug}` : null);

	// The claimed book carries its language (set by the full player), so the
	// mini player labels follow it too.
	const t = $derived(
		book?.vietnamese
			? {
					nowPlaying: 'Đang phát',
					open: (title) => `Mở ${title}`,
					previous: 'Chương trước',
					play: 'Phát',
					pause: 'Tạm dừng',
					next: 'Chương sau',
					close: 'Đóng trình phát nhỏ',
					closeTitle: 'Đóng',
					seek: 'Tua trong chương'
				}
			: {
					nowPlaying: 'Now playing',
					open: (title) => `Open ${title}`,
					previous: 'Previous chapter',
					play: 'Play',
					pause: 'Pause',
					next: 'Next chapter',
					close: 'Close mini player',
					closeTitle: 'Close',
					seek: 'Seek within chapter'
				}
	);

	/** Value shown while the listener drags the timeline. */
	let scrub = $state(null);
	/** Ratio from the last pointer event; committed on release, so a pointerup
	 * that arrives without usable coordinates cannot corrupt the seek. */
	let pendingRatio = $state(null);

	const percent = $derived(percentOf(scrub ?? engine?.time ?? 0, engine?.displayDuration ?? 0));
	const bufferedPercent = $derived(percentOf(engine?.buffered ?? 0, engine?.displayDuration ?? 0));

	// The timeline maps pointer positions the same way the full player's bar
	// does: the visible track maps exactly, without thumb-width correction.
	function trackRatio(event) {
		const rect = event.currentTarget.getBoundingClientRect();
		return Math.min(1, Math.max(0, (event.clientX - rect.left) / rect.width));
	}

	function previewScrub(event) {
		pendingRatio = trackRatio(event);
		scrub = pendingRatio * (engine?.displayDuration ?? 0);
	}

	function commitSeek() {
		if (pendingRatio != null) engine?.seekToRatio(pendingRatio);
		pendingRatio = null;
		scrub = null;
	}

	function cancelSeek() {
		pendingRatio = null;
		scrub = null;
	}

	/** The invisible range input reports 0..1000 so keyboard seeks land cleanly. */
	function commitSeekInput(event) {
		engine?.seekToRatio(Number(event.currentTarget.value) / 1000);
		scrub = null;
	}

	// Hidden while the reader is on this book's page: the full player is already
	// there, so a second set of controls would just be noise.
	const visible = $derived(session.miniVisible);

	// Marquee: the chapter line only scrolls when it actually overflows, and the
	// duration grows with the distance so the speed stays readable either way.
	let lineWidth = $state(0);
	let textWidth = $state(0);
	const overflow = $derived(Math.max(0, textWidth - lineWidth));
	const marqueeSeconds = $derived(Math.round(6 + overflow / 25));
</script>

{#if visible && engine && book}
	<!-- Pinned to the bottom corner, below the "to top" button: the button
	     yields the space instead (it moves up while this card is showing).
	     Deliberately under it in the stack too (z-40 vs z-50), so the button and
	     its shadow stay crisp instead of sitting under this card's shadow.
	     Styled as one of the site's white cards so it reads as part of the page
	     instead of blending into dark sections. -->
	<aside
		in:fly={{ y: 16, duration: 200 }}
		out:fly={{ y: 16, duration: 150 }}
		class="fixed bottom-4 right-4 z-40 w-[min(23rem,calc(100vw-2rem))] overflow-hidden rounded-xl border-3 border-dark/50 bg-white text-dark shadow-xl"
		aria-label={t.nowPlaying}
		lang={book?.vietnamese ? 'vi' : undefined}
	>
		{#if book.coverUrl}
			<!-- The cover IS the card's background: full-bleed, centered, with a
			     white veil on top so the text stays readable over busy artwork.
			     Tune the veil alpha (and with it the artwork's strength). -->
			<span
				aria-hidden="true"
				class="absolute inset-0 bg-cover bg-center"
				style:background-image={`linear-gradient(rgba(255,255,255,0.65), rgba(255,255,255,0.65)), url("${book.coverUrl}")`}
			></span>
		{/if}

		<!-- Timeline: the same pointer-driven scrubber as the full player,
		     compressed to a thin strip that doubles as the card's top rule. -->
		<div
			class="relative z-10 h-4 flex items-center cursor-pointer touch-none"
			onpointerdown={(event) => {
				event.currentTarget.setPointerCapture(event.pointerId);
				previewScrub(event);
			}}
			onpointermove={(event) => {
				if (event.buttons > 0) previewScrub(event);
			}}
			onpointerup={commitSeek}
			onpointercancel={cancelSeek}
		>
			<div class="absolute inset-x-0 h-1.5 rounded-full bg-dark/15 overflow-hidden">
				<div class="h-full bg-dark/25" style:width={`${bufferedPercent}%`}></div>
			</div>
			<div
				class="absolute left-0 h-1.5 rounded-full bg-primary pointer-events-none"
				style:width={`${percent}%`}
			></div>
			<div
				class="absolute h-3.5 w-3.5 rounded-full bg-white border-2 border-primary shadow pointer-events-none"
				style:left={`calc(${percent}% - 7px)`}
			></div>
			<input
				type="range"
				min="0"
				max="1000"
				step="1"
				class="absolute inset-0 w-full opacity-0 pointer-events-none"
				aria-label={t.seek}
				value={Math.round(percent * 10)}
				oninput={(event) =>
					(scrub = (Number(event.currentTarget.value) / 1000) * (engine?.displayDuration ?? 0))}
				onchange={commitSeekInput}
			/>
		</div>

		<div class="relative z-10 flex items-center gap-2 p-2 md:gap-3 md:p-3">
			<div class="min-w-0 grow">
				{#if href}
					<a
						{href}
						class="block truncate text-sm font-semibold text-dark md:text-base"
						title={t.open(book.title)}
					>
						{book.title}
					</a>
				{:else}
					<p class="truncate text-sm font-semibold md:text-base">{book.title}</p>
				{/if}
				<!-- Only the number and the title: the card is too narrow to spend
				     characters on the word "Chapter". Long titles drift left and
				     right instead of being cut off. -->
				<p
					class="overflow-hidden text-xs whitespace-nowrap text-dark/80 md:text-sm"
					bind:clientWidth={lineWidth}
				>
					<span
						class="inline-block whitespace-nowrap motion-reduce:animate-none"
						class:animate-marquee={overflow > 1}
						style="--marquee-shift: -{overflow}px; --marquee-duration: {marqueeSeconds}s"
						bind:clientWidth={textWidth}
					>
						{track?.number ?? 1} - {track?.title ?? ''}
					</span>
				</p>
				<p class="text-xs tabular-nums text-dark/50 md:text-sm">
					{formatClock(engine.time)} / {formatClock(engine.displayDuration)}
				</p>
			</div>

			<button
				type="button"
				class="grid h-8 w-8 shrink-0 place-items-center rounded-full bg-dark/10 transition-colors hover:bg-dark/20 disabled:opacity-30 md:h-9 md:w-9"
				disabled={!engine.hasPrevious}
				onclick={() => engine.previous()}
				aria-label={t.previous}
				title={t.previous}
			>
				<svg class="h-4 w-4 fill-dark md:h-5 md:w-5" viewBox="0 0 24 24">
					<path d="M7 6h2v12H7zm3 6l9 6V6z" />
				</svg>
			</button>

			<button
				type="button"
				class="grid h-10 w-10 shrink-0 place-items-center rounded-full transition-colors md:h-11 md:w-11 {engine.playing
					? 'bg-accent-red hover:bg-accent-red-dark'
					: 'bg-accent-green hover:bg-accent-green-dark'}"
				onclick={() => engine.toggle()}
				aria-label={engine.playing ? t.pause : t.play}
				title={engine.playing ? t.pause : t.play}
			>
				{#if engine.playing}
					<svg class="h-5 w-5 fill-white md:h-6 md:w-6" viewBox="0 0 24 24">
						<path d="M7 5h4v14H7zm6 0h4v14h-4z" />
					</svg>
				{:else}
					<svg class="h-5 w-5 fill-white md:h-6 md:w-6" viewBox="0 0 24 24">
						<path d="M8 5l11 7-11 7z" />
					</svg>
				{/if}
			</button>

			<button
				type="button"
				class="grid h-8 w-8 shrink-0 place-items-center rounded-full bg-dark/10 transition-colors hover:bg-dark/20 disabled:opacity-30 md:h-9 md:w-9"
				disabled={!engine.hasNext}
				onclick={() => engine.next()}
				aria-label={t.next}
				title={t.next}
			>
				<svg class="h-4 w-4 fill-dark md:h-5 md:w-5" viewBox="0 0 24 24">
					<path d="M15 6h2v12h-2zM5 6l9 6-9 6z" />
				</svg>
			</button>

			<button
				type="button"
				class="grid h-6 w-6 shrink-0 place-items-center rounded-full text-dark/40 transition-colors hover:bg-dark/10 hover:text-dark"
				onclick={() => session.close()}
				aria-label={t.close}
				title={t.closeTitle}
			>
				<svg
					class="h-4 w-4"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					stroke-linecap="round"
				>
					<path d="M18 6 6 18M6 6l12 12" />
				</svg>
			</button>
		</div>
	</aside>
{/if}
