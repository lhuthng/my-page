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
</script>

<div
	class="sticky-below-header hidden sm:block sticky self-start mb-2 lg:mb-4 w-14 min-w-14 lg:w-46 lg:min-w-46 drop-shadow-sm space-y-2 lg:space-y-4 transition-transform duration-200"
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
	<div class="flex flex-col gap-2 bg-white p-1 lg:p-2 rounded-xl">
		<div class="not-lg:w-10 flex flex-col" class:w-10={compact}>
			<span class="block not-lg:hidden text-center" class:hidden={compact}>
				Connect with me on:
			</span>
			<div
				class="flex w-full not-lg:flex-col lg:justify-evenly items-center [&>a]:hover:fill-dark/90"
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
		</div>
		<div class="not-lg:hidden px-2 flex flex-col font-medium" class:hidden={compact}>
			<span class="font-normal">more:</span>
			<ul class="list-disc list-inside">
				<li>
					<a href={publicEnv.PUBLIC_PORTFORLIO_URL ?? '/'}>Portfolio</a>
				</li>
				<li><a href="/">About</a></li>
			</ul>
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
	   below `lg`, applied at every breakpoint. Scoped specificity outranks the
	   `lg:w-46` utility, so no !important games. */
	.collapsed {
		width: 3.5rem;
		min-width: 3.5rem;
	}
</style>
