<script>
	import { page } from '$app/stores';
	import { saveLogin } from '$lib/auth/user.svelte.js';
	import Footer from '$lib/components/shell/Footer.svelte';
	import Header from '$lib/components/shell/Header.svelte';
	import NavigationSideBar from '$lib/components/shell/NavigationSideBar.svelte';
	import { onMount } from 'svelte';
	import '../app.css';
	import { el } from '$lib/dom/elements.svelte.js';
	import { fade } from 'svelte/transition';
	import ToTop from '$lib/components/shell/ToTop.svelte';
	import { win } from '$lib/dom/windows.svelte.js';
	import { innerWidth } from 'svelte/reactivity/window';

	let { data, children } = $props();

	let route = $derived($page.url.pathname.split('/')[1]);
	let pDiv = $state();
	let mDiv = $state();
	let scrollTarget = $state();

	const ignoreRoutes = ['login', 'verify-email'];

	$effect(() => {
		win.width = innerWidth.current;
	});

	$effect(() => {
		if (data?.accessToken?.token) {
			saveLogin({
				username: data.user?.username,
				displayName: data.user?.displayName,
				token: data.accessToken.token,
				tokenType: data.accessToken.type,
				role: data.user?.role,
				avatarUrl: data.user?.avatarUrl
			});
		}
	});

	onMount(() => {
		el.pbody = pDiv;
		el.mbody = mDiv;
	});
</script>

<svelte:head>
	<meta property="og:site_name" content="Huu Thang's Blog" />

	<link rel="icon" href="/favicon.ico" />
</svelte:head>

<div class="fixed w-dvw h-dvh pointer-events-none z-11" bind:this={mDiv}></div>
<div class="relative flex flex-col min-h-screen z-10">
	<div class="absolute pointer-events-none inset-0 z-50" bind:this={pDiv}></div>
	<Header />
	<main class="grow" bind:this={scrollTarget}>
		<div class="relative flex gap-2 lg:gap-4 w-cap">
			{#if !ignoreRoutes.includes(route)}
				<NavigationSideBar {route} />
			{/if}
			<div class="w-full not-lg:overflow-x-hidden">
				{@render children?.()}
			</div>
		</div>
		<ToTop {scrollTarget} />
	</main>
	<Footer />
</div>

<style lang="postcss">
	@reference "../app.css";

	main {
		@apply flex bg-background pt-32 text-dark not-lg:pt-16;
	}
</style>
