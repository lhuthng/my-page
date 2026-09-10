<script>
	import {
		blankVariant,
		parseManifestModel,
		serializeManifestModel
	} from '$lib/features/v86/manifest-editor.js';

	let { value = $bindable(''), disabled = false } = $props();

	// The string stays the source of truth: parse on load, serialize on every
	// edit. If the bound value changes from outside (draft restore, save
	// refresh), re-parse it.
	let model = $state(parseManifestModel(value ?? ''));
	let emitted = $state(value ?? '');

	function commit(next) {
		model = next;
		emitted = serializeManifestModel(next);
		value = emitted;
	}

	$effect(() => {
		if ((value ?? '') !== emitted) {
			model = parseManifestModel(value ?? '');
			emitted = value ?? '';
		}
	});

	function setVariant(index, field, next) {
		commit({
			...model,
			variants: model.variants.map((variant, i) =>
				i === index ? { ...variant, [field]: next } : variant
			)
		});
	}

	function addVariant() {
		commit({ ...model, variants: [...model.variants, blankVariant()] });
	}

	function removeVariant(index) {
		if (model.variants.length <= 1) return;
		commit({ ...model, variants: model.variants.filter((_, i) => i !== index) });
	}

	function setPath(index, next) {
		commit({ ...model, savePaths: model.savePaths.map((path, i) => (i === index ? next : path)) });
	}

	function addPath() {
		commit({ ...model, savePaths: [...model.savePaths, ''] });
	}

	function removePath(index) {
		commit({ ...model, savePaths: model.savePaths.filter((_, i) => i !== index) });
	}

	function removeUnknown(index) {
		commit({ ...model, unknown: model.unknown.filter((_, i) => i !== index) });
	}

	const input =
		'w-full rounded-xl px-3 py-2 text-dark outline-none border-2 border-dark transition-colors focus:bg-primary focus:text-white disabled:opacity-60';
	const addBtn =
		'flex h-7 w-7 items-center justify-center rounded-lg border border-background bg-background/40 text-lg leading-none text-dark transition-colors hover:bg-background/60 disabled:opacity-50';
	const removeBtn =
		'flex h-7 w-7 shrink-0 items-center justify-center rounded-lg border border-accent-red/30 bg-accent-red-light-4 text-accent-red transition-colors hover:bg-accent-red-light-3 disabled:opacity-50';
</script>

<div class="flex flex-col gap-3">
	<div class="flex items-center justify-between">
		<span class="text-sm font-medium text-dark/70">Launch variants</span>
		<button type="button" class={addBtn} {disabled} title="Add variant" onclick={addVariant}>
			+
		</button>
	</div>
	{#each model.variants as variant, index (index)}
		<div class="flex flex-col gap-2 rounded-xl border border-dark/15 bg-dark/5 p-3">
			<div class="flex items-center justify-between">
				<span class="text-sm font-medium text-dark/70">Variant {index + 1}</span>
				{#if model.variants.length > 1}
					<button
						type="button"
						class={removeBtn}
						{disabled}
						title="Remove variant {index + 1}"
						onclick={() => removeVariant(index)}
					>
						✕
					</button>
				{/if}
			</div>
			<label class="flex flex-col gap-1">
				<span class="text-sm font-medium text-dark/60">Name</span>
				<input
					class={input}
					value={variant.name}
					{disabled}
					oninput={(e) => setVariant(index, 'name', e.currentTarget.value)}
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-sm font-medium text-dark/60">Executable</span>
				<input
					class={input}
					placeholder="game.exe — relative to the game drive"
					value={variant.exe}
					{disabled}
					oninput={(e) => setVariant(index, 'exe', e.currentTarget.value)}
				/>
			</label>
			<label class="flex flex-col gap-1">
				<span class="text-sm font-medium text-dark/60">Arguments</span>
				<input
					class={input}
					value={variant.args}
					{disabled}
					oninput={(e) => setVariant(index, 'args', e.currentTarget.value)}
				/>
			</label>
		</div>
	{/each}

	<div class="grid grid-cols-2 gap-2">
		<label class="flex flex-col gap-1">
			<span class="text-sm font-medium text-dark/60">Delay (ms)</span>
			<input
				class={input}
				inputmode="numeric"
				placeholder="1000"
				value={model.delayMs}
				{disabled}
				oninput={(e) => commit({ ...model, delayMs: e.currentTarget.value.trim() })}
			/>
		</label>
		<label class="flex flex-col gap-1">
			<span class="text-sm font-medium text-dark/60">Mouse speed</span>
			<input
				class={input}
				inputmode="decimal"
				placeholder="1.0"
				value={model.mouseSpeed}
				{disabled}
				oninput={(e) => commit({ ...model, mouseSpeed: e.currentTarget.value.trim() })}
			/>
		</label>
	</div>
	<label class="flex items-center gap-2">
		<input
			type="checkbox"
			class="accent-primary h-4 w-4"
			checked={model.revertMouseY}
			{disabled}
			onchange={(e) => commit({ ...model, revertMouseY: e.currentTarget.checked })}
		/>
		<span
			class="text-sm text-dark/75 underline decoration-dashed underline-offset-2 cursor-help"
			title="Inverts the mouse's Y axis."
		>
			Revert mouse Y
		</span>
	</label>

	<div class="flex flex-col gap-2">
		<div class="flex items-center justify-between">
			<span class="text-sm font-medium text-dark/70">Save paths</span>
			<button type="button" class={addBtn} {disabled} title="Add save path" onclick={addPath}>
				+
			</button>
		</div>
		{#each model.savePaths as path, index (index)}
			<div class="flex items-center gap-1.5">
				<input
					class={input}
					placeholder="Save0001.dat"
					value={path}
					{disabled}
					oninput={(e) => setPath(index, e.currentTarget.value)}
				/>
				<button
					type="button"
					class={removeBtn}
					{disabled}
					title="Remove save path"
					onclick={() => removePath(index)}
				>
					✕
				</button>
			</div>
		{/each}
	</div>

	{#if model.unknown.length > 0}
		<div class="flex flex-col gap-2">
			<span class="text-sm font-medium text-dark/70">Other keys</span>
			{#each model.unknown as entry, index (index)}
				<div class="flex items-center gap-1.5">
					<span class="w-28 shrink-0 truncate font-mono text-sm text-dark/60">{entry.key}</span>
					<input
						class={input}
						value={entry.value}
						{disabled}
						oninput={(e) =>
							commit({
								...model,
								unknown: model.unknown.map((u, i) =>
									i === index ? { ...u, value: e.currentTarget.value } : u
								)
							})}
					/>
					<button
						type="button"
						class={removeBtn}
						{disabled}
						title="Remove key"
						onclick={() => removeUnknown(index)}
					>
						✕
					</button>
				</div>
			{/each}
		</div>
	{/if}
</div>
