<script>
	import { fly } from 'svelte/transition';
	import Book from '$lib/components/svgs/Book.svelte';
	import { audiobookSession } from '$lib/players/AudiobookSession.svelte.js';
	import { formatClock, percentOf } from '$lib/utils/duration.js';

	const session = audiobookSession;

	const engine = $derived(session.current?.engine ?? null);
	const book = $derived(session.current?.book ?? null);
	const track = $derived(engine?.current ?? null);
	const percent = $derived(percentOf(engine?.time ?? 0, engine?.displayDuration ?? 0));
	const href = $derived(book?.slug ? `/audiobooks/${book.slug}` : null);

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
		class="fixed bottom-4 right-4 z-40 w-[min(23rem,calc(100vw-2rem))] overflow-hidden rounded-xl border-2 border-dark/40 bg-white text-dark shadow-xl"
		aria-label="Now playing"
	>
		<div class="h-1 w-full bg-dark/15">
			<div class="h-full bg-primary" style:width={`${percent}%`}></div>
		</div>

		<div class="flex items-center gap-2 p-2">
			{#if href}
				<a {href} class="shrink-0 no-underline!" title="Open {book.title}">
					{#if book.coverUrl}
						<!-- Zoomed inside a fixed box: the artwork is what identifies the
						     book, and the full cover shrinks to an unreadable speck at
						     this size. -->
						<span class="block h-11 w-11 overflow-hidden rounded-lg bg-dark/10">
							<img src={book.coverUrl} alt="" class="h-full w-full scale-[1.35] object-cover" />
						</span>
					{:else}
						<span class="grid h-11 w-11 place-items-center rounded-lg bg-dark/10">
							<Book class="h-6 w-6 text-dark/50" />
						</span>
					{/if}
				</a>
			{/if}

			<div class="min-w-0 grow">
				{#if href}
					<a {href} class="block truncate text-sm font-semibold text-dark">
						{book.title}
					</a>
				{:else}
					<p class="truncate text-sm font-semibold">{book.title}</p>
				{/if}
				<!-- Only the number and the title: the card is too narrow to spend
				     characters on the word "Chapter". Long titles drift left and
				     right instead of being cut off. -->
				<p
					class="overflow-hidden text-xs whitespace-nowrap text-dark/60"
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
				<p class="text-xs tabular-nums text-dark/50">
					{formatClock(engine.time)} / {formatClock(engine.displayDuration)}
				</p>
			</div>

			<button
				type="button"
				class="grid h-8 w-8 shrink-0 place-items-center rounded-full bg-dark/10 transition-colors hover:bg-dark/20 disabled:opacity-30"
				disabled={!engine.hasPrevious}
				onclick={() => engine.previous()}
				aria-label="Previous chapter"
				title="Previous chapter"
			>
				<svg class="h-4 w-4 fill-dark" viewBox="0 0 24 24">
					<path d="M7 6h2v12H7zm3 6l9 6V6z" />
				</svg>
			</button>

			<button
				type="button"
				class="grid h-10 w-10 shrink-0 place-items-center rounded-full transition-colors {engine.playing
					? 'bg-accent-red hover:bg-accent-red-dark'
					: 'bg-accent-green hover:bg-accent-green-dark'}"
				onclick={() => engine.toggle()}
				aria-label={engine.playing ? 'Pause' : 'Play'}
				title={engine.playing ? 'Pause' : 'Play'}
			>
				{#if engine.playing}
					<svg class="h-5 w-5 fill-white" viewBox="0 0 24 24">
						<path d="M7 5h4v14H7zm6 0h4v14h-4z" />
					</svg>
				{:else}
					<svg class="h-5 w-5 fill-white" viewBox="0 0 24 24">
						<path d="M8 5l11 7-11 7z" />
					</svg>
				{/if}
			</button>

			<button
				type="button"
				class="grid h-8 w-8 shrink-0 place-items-center rounded-full bg-dark/10 transition-colors hover:bg-dark/20 disabled:opacity-30"
				disabled={!engine.hasNext}
				onclick={() => engine.next()}
				aria-label="Next chapter"
				title="Next chapter"
			>
				<svg class="h-4 w-4 fill-dark" viewBox="0 0 24 24">
					<path d="M15 6h2v12h-2zM5 6l9 6-9 6z" />
				</svg>
			</button>

			<button
				type="button"
				class="grid h-6 w-6 shrink-0 place-items-center rounded-full text-dark/40 transition-colors hover:bg-dark/10 hover:text-dark"
				onclick={() => session.close()}
				aria-label="Close mini player"
				title="Close"
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
