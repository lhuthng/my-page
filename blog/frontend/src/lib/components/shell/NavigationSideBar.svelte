<script>
	import { authState } from '$lib/auth/user.svelte.js';
	import { env as publicEnv } from '$env/dynamic/public';
	import SideBarItem from './SideBarItem.svelte';
	import SidebarToggle from './SidebarToggle.svelte';
	import { navCollapse } from '$lib/dom/nav-collapse.svelte.js';
	import { publicNavItems } from './navigation.js';
	import FacebookButton from './buttons/FacebookButton.svelte';
	import GithubButton from './buttons/GithubButton.svelte';
	import LinkedinButton from './buttons/LinkedinButton.svelte';

	let { route } = $props();

	// The user's manual collapse (SidebarToggle) forces the compact rail at
	// `lg` and up; below that the responsive classes already decide the shape.
	let compact = $derived(navCollapse.collapsed);

	// The three social icons re-flow between the expanded row and the compact
	// column. Flex-direction cannot animate, so the move is FLIP-ed: capture
	// each icon's box before the class change, then glide it from where it was
	// to where it landed.
	let connectCard = $state();
	let iconRects = null;

	$effect.pre(() => {
		compact;
		if (!connectCard) return;
		iconRects = new Map(
			[...connectCard.querySelectorAll('a')].map((el) => [el, el.getBoundingClientRect()])
		);
	});

	$effect(() => {
		compact;
		if (!connectCard || !iconRects) return;
		for (const [el, rect] of iconRects) {
			const box = el.getBoundingClientRect();
			const dx = rect.left - box.left;
			const dy = rect.top - box.top;
			if (dx || dy) {
				el.style.transition = 'none';
				el.style.transform = `translate(${dx}px, ${dy}px)`;
				el.getBoundingClientRect(); // flush, so the glide below is measured
				el.style.transition = 'transform 250ms cubic-bezier(0.2, 0.8, 0.2, 1)';
				el.style.transform = '';
			}
		}
		iconRects = null;
	});
</script>

<div
	class="sticky-below-header hidden sm:block sticky self-start mb-2 lg:mb-4 w-14 min-w-14 lg:w-46 lg:min-w-46 drop-shadow-sm space-y-2 lg:space-y-4 transition-[width,min-width] duration-300 ease-out"
	class:collapsed={compact}
>
	<ul class="space-y-2 bg-white p-2 rounded-xl" id="side-bar">
		<!-- The toggle row exists only from lg up: below it the rail is already
		     compact, and an empty row would leave a phantom gap in the list. -->
		<li class="hidden lg:block"><SidebarToggle /></li>
		{#each publicNavItems as item (item.path)}
			{#if !item.secret || authState.isMod}
				<SideBarItem
					icon={item.icon}
					label={item.label}
					href={item.path}
					active={item.route === route}
					{compact}
				/>
			{/if}
		{/each}
	</ul>
	<div class="flex flex-col gap-2 bg-white p-1 lg:p-2 rounded-xl" bind:this={connectCard}>
		<!-- The heading and the extra links fold vertically through the same
		     grid trick as the item labels; the social icons glide between their
		     row and column spots (FLIP above). -->
		<div
			class="grid transition-[grid-template-rows] duration-200 ease-out"
			style:grid-template-rows={compact ? '0fr' : '1fr'}
		>
			<span class="block overflow-hidden not-lg:hidden text-center">Connect with me on:</span>
		</div>
		<div
			class="flex w-full items-center not-lg:flex-col [&>a]:hover:fill-dark/90"
			class:justify-evenly={!compact}
			class:flex-col={compact}
		>
			<FacebookButton as="a" class="w-10 fill-dark" href="https://www.facebook.com/lhuthng/" />
			<GithubButton as="a" class="w-10 fill-dark" href="https://github.com/lhuthng" />
			<LinkedinButton
				as="a"
				class="w-10 fill-dark"
				href="https://www.linkedin.com/in/huuthangle/"
			/>
		</div>
		<div
			class="grid transition-[grid-template-rows] duration-200 ease-out"
			style:grid-template-rows={compact ? '0fr' : '1fr'}
		>
			<div class="overflow-hidden px-2 not-lg:hidden">
				<span class="font-normal block">more:</span>
				<ul class="list-disc list-inside">
					<li>
						<a href={publicEnv.PUBLIC_PORTFORLIO_URL ?? '/'}>Portfolio</a>
					</li>
					<li><a href="/">About</a></li>
				</ul>
			</div>
		</div>
	</div>
</div>

<style lang="postcss">
	@reference "../../../app.css";

	/* Item styling lives in SideBarItem so both rails stay identical; this
	   only keeps the card's non-item links (connect, more) underline-free. */
	a {
		@apply no-underline!;
	}

	/* The manual collapse: the compact width the responsive classes only give
	   below `lg`, applied at every breakpoint — and animated, so the rail
	   glides instead of snapping. Scoped specificity outranks the `lg:w-46`
	   utility, so no !important games. */
	.collapsed {
		width: 3.5rem;
		min-width: 3.5rem;
	}
</style>
