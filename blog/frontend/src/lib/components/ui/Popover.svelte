<script>
	import { fade } from 'svelte/transition';

	let {
		open = $bindable(false),
		position = 'bottom',
		align = 'center',
		offset = 0,
		class: className = '',
		panelClass = '',
		children,
		anchor
	} = $props();

	let wrapperEl = $state();
	let panelEl = $state();
	let panelWidth = $state(0);
	let panelHeight = $state(0);

	const panelStyle = $derived.by(() => {
		const aw = wrapperEl?.offsetWidth ?? 0;
		const ah = wrapperEl?.offsetHeight ?? 0;
		let left = null;
		let top = null;
		let right = null;
		let bottom = null;

		if (position === 'top' || position === 'bottom') {
			if (position === 'bottom') top = ah + offset;
			else bottom = ah + offset;
			if (align === 'right') right = 0;
			else if (align === 'left') left = 0;
			else left = (aw - panelWidth) / 2;
		} else if (position === 'left' || position === 'right') {
			if (position === 'right') left = aw + offset;
			else right = aw + offset;
			if (align === 'bottom') bottom = 0;
			else if (align === 'top') top = 0;
			else top = (ah - panelHeight) / 2;
		} else {
			left = (aw - panelWidth) / 2;
			top = (ah - panelHeight) / 2;
		}

		const parts = ['position:absolute;'];
		if (left != null) parts.push(`left:${left}px;`);
		if (top != null) parts.push(`top:${top}px;`);
		if (right != null) parts.push(`right:${right}px;`);
		if (bottom != null) parts.push(`bottom:${bottom}px;`);
		return parts.join(' ');
	});

	$effect(() => {
		if (!open || !panelEl) return;
		panelWidth = panelEl.offsetWidth;
		panelHeight = panelEl.offsetHeight;
		const observer = new ResizeObserver(() => {
			panelWidth = panelEl.offsetWidth;
			panelHeight = panelEl.offsetHeight;
		});
		observer.observe(panelEl);
		return () => observer.disconnect();
	});

	$effect(() => {
		if (!open) return;
		const onPointerDown = (event) => {
			if (wrapperEl && !wrapperEl.contains(event.target)) open = false;
		};
		const onKeyDown = (event) => {
			if (event.key === 'Escape') open = false;
		};
		window.addEventListener('pointerdown', onPointerDown);
		window.addEventListener('keydown', onKeyDown);
		return () => {
			window.removeEventListener('pointerdown', onPointerDown);
			window.removeEventListener('keydown', onKeyDown);
		};
	});
</script>

<div bind:this={wrapperEl} class={`relative inline-block ${className}`}>
	{@render anchor?.()}

	{#if open}
		<div
			bind:this={panelEl}
			in:fade={{ duration: 100 }}
			class={`absolute z-50 ${panelClass}`}
			style={panelStyle}
		>
			{@render children?.()}
		</div>
	{/if}
</div>
