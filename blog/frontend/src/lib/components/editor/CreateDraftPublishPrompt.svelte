<script>
	let {
		show = false,
		kind = 'post',
		busy = false,
		error = '',
		onconfirm = async () => {},
		oncancel = () => {}
	} = $props();
</script>

{#if show}
	<div class="fixed inset-0 z-30 grid place-items-center px-4">
		<div
			class="absolute inset-0 bg-dark/50"
			onclick={busy ? undefined : oncancel}
			role="none"
		></div>
		<div class="relative z-31 w-full max-w-md rounded-3xl bg-white p-6 shadow-2xl">
			<div class="space-y-3">
				<h2 class="text-2xl">Saved as Draft</h2>
				<p class="text-dark/80">
					This {kind} was created successfully and is still a draft. Would you like to publish it now?
				</p>
				{#if error}
					<p class="text-sm text-accent-red">{error}</p>
				{/if}
			</div>
			<div class="mt-6 flex justify-end gap-3">
				<div class="duo-btn" data-duo-color="blue">
					<button onclick={oncancel} disabled={busy}>No</button>
				</div>
				<div class="duo-btn" data-duo-color="green">
					<button onclick={onconfirm} disabled={busy}>
						{busy ? 'Publishing…' : 'Yes, publish'}
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}
