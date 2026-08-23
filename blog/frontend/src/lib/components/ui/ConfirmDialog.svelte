<script>
	let {
		open = false,
		title = 'Are you sure?',
		description = '',
		confirmLabel = 'Delete',
		cancelLabel = 'Cancel',
		confirmColor = 'red',
		requireTyping = null,
		typedValue = $bindable(''),
		busy = false,
		onconfirm,
		oncancel
	} = $props();

	let canConfirm = $derived(
		!requireTyping || typedValue.trim() === requireTyping
	);

	function handleBackdrop(e) {
		if (e.target === e.currentTarget && !busy) oncancel?.();
	}
</script>

{#if open}
	<div
		class="fixed inset-0 z-40 flex items-center justify-center bg-dark/45 p-4"
		onclick={handleBackdrop}
		role="presentation"
	>
		<div
			role="dialog"
			aria-modal="true"
			aria-labelledby="confirm-title"
			class="w-full max-w-md rounded-2xl bg-white p-6 shadow-2xl"
		>
			<h3 id="confirm-title" class="text-lg font-semibold text-dark">{title}</h3>
			{#if description}
				<p class="mt-2 text-sm leading-relaxed text-dark/70">{description}</p>
			{/if}
			{#if requireTyping}
				<p class="mt-4 text-sm text-dark/70">
					Type <span class="font-mono font-semibold text-dark">{requireTyping}</span> to confirm.
				</p>
				<input
					type="text"
					bind:value={typedValue}
					placeholder={requireTyping}
					class="mt-2 w-full rounded-lg border border-dark/20 px-3 py-2 text-sm focus:border-accent-red focus:outline-none"
					disabled={busy}
				/>
			{/if}
			<div class="mt-6 flex justify-end gap-3">
				<button
					onclick={oncancel}
					disabled={busy}
					class="rounded-full border border-dark/20 px-5 py-2 text-sm font-medium text-dark hover:bg-dark/5 disabled:opacity-50"
				>
					{cancelLabel}
				</button>
				<button
					onclick={onconfirm}
					disabled={busy || !canConfirm}
					class="rounded-full px-5 py-2 text-sm font-medium text-white disabled:opacity-50
						{confirmColor === 'red' ? 'bg-accent-red hover:bg-accent-red/90' : 'bg-dark hover:bg-dark/90'}"
				>
					{busy ? 'Deleting…' : confirmLabel}
				</button>
			</div>
		</div>
	</div>
{/if}
