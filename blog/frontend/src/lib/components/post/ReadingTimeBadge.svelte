<script>
	let {
		minutes = 0,
		variant = 'ticket',
		size = 'md',
		label = undefined,
		class: className = ''
	} = $props();

	let tier = $derived(
		minutes <= 0
			? null
			: minutes < 6
				? 'green'
				: minutes < 12
					? 'blue'
					: minutes < 25
						? 'yellow'
						: minutes < 45
							? 'orange'
							: 'red'
	);

	let text = $derived(label ?? `${minutes} min`);
</script>

{#if tier !== null}
	{#if variant === 'ticket'}
		<div class="reading-ticket {tier} {size} {className}" title={`${minutes} min read`}>
			<svg class="ticket-clock" viewBox="0 0 24 24" aria-hidden="true" fill="currentColor">
				<path
					d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm0 18a8 8 0 1 1 0-16 8 8 0 0 1 0 16Zm.5-13h-1v6.2l4.5 2.7.5-.8-4-2.4V7Z"
				></path>
			</svg>
			<span class="ticket-text">{text}</span>
		</div>
	{:else}
		<span
			class="reading-inline {tier} {size} inline-flex items-center gap-1 align-middle {className}"
			title={`${minutes} min read`}
		>
			<svg class="h-3.5 w-3.5" viewBox="0 0 24 24" aria-hidden="true" fill="currentColor">
				<path
					d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm0 18a8 8 0 1 1 0-16 8 8 0 0 1 0 16Zm.5-13h-1v6.2l4.5 2.7.5-.8-4-2.4V7Z"
				></path>
			</svg>
			<span>{text}</span>
		</span>
	{/if}
{/if}

<style lang="postcss">
	@reference "../../../app.css";

	.reading-ticket {
		@apply flex items-center gap-0.5 rounded-md font-semibold;
		box-shadow: 0 1px 3px rgb(0 0 0 / 0.2);
		clip-path: polygon(
			calc(0% + 0.5rem) 0%,
			100% 0%,
			100% calc(50% - 0.3rem),
			calc(100% - 0.5rem) 50%,
			100% calc(50% + 0.3rem),
			100% 100%,
			calc(0% + 0.5rem) 100%,
			0% calc(50% + 0.3rem),
			0.5rem 50%,
			0% calc(50% - 0.3rem)
		);
	}

	.ticket-clock {
		@apply h-3.5 w-3.5;
	}

	.ticket-text {
		@apply leading-none;
	}

	.reading-ticket.md {
		@apply gap-1 px-2.5 py-1 text-sm;
	}

	.reading-ticket.lg {
		@apply gap-1 px-3 py-1.5 text-base;
	}

	.reading-ticket.green {
		@apply border border-accent-green bg-accent-green-light-2 text-accent-green;
	}

	.reading-ticket.blue {
		@apply border border-accent-blue bg-accent-blue-light-2 text-accent-blue;
	}

	.reading-ticket.yellow {
		@apply border border-accent-yellow bg-accent-yellow-light-2 text-accent-yellow;
	}

	.reading-ticket.orange {
		@apply border border-accent-orange bg-accent-orange-light-2 text-accent-orange;
	}

	.reading-ticket.red {
		@apply border border-accent-red bg-accent-red-light-2 text-accent-red;
	}

	.reading-inline.md {
		@apply text-sm;
	}

	.reading-inline.lg {
		@apply text-base;
	}

	.reading-inline.green {
		@apply text-accent-green;
	}

	.reading-inline.blue {
		@apply text-accent-blue;
	}

	.reading-inline.yellow {
		@apply text-accent-yellow;
	}

	.reading-inline.orange {
		@apply text-accent-orange;
	}

	.reading-inline.red {
		@apply text-accent-red;
	}
</style>
