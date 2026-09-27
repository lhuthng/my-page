<script>
	import { authState } from '$lib/auth/user.svelte.js';
	import { env as publicEnv } from '$env/dynamic/public';
	import SideBarItem from './SideBarItem.svelte';
	import AboutButton from './buttons/AboutButton.svelte';
	import AudiobookButton from './buttons/AudiobookButton.svelte';
	import BlogButton from './buttons/BlogButton.svelte';
	import DashboardButton from './buttons/DashboardButton.svelte';
	import FacebookButton from './buttons/FacebookButton.svelte';
	import GithubButton from './buttons/GithubButton.svelte';
	import HomeButton from './buttons/HomeButton.svelte';
	import LinkedinButton from './buttons/LinkedinButton.svelte';
	import ProjectButton from './buttons/ProjectButton.svelte';
	import SeriesButton from './buttons/SeriesButton.svelte';

	let { route } = $props();

	const routes = [
		[HomeButton, 'Home', '/', ''],
		[BlogButton, 'Posts', '/posts', 'posts'],
		[ProjectButton, 'Projects', '/projects', 'projects'],
		[SeriesButton, 'Series', '/series', 'series'],
		[AudiobookButton, 'Audiobooks', '/audiobooks', 'audiobooks'],
		[AboutButton, 'About', '/about', 'about'],
		[DashboardButton, 'Dashboard', '/dashboard', 'dashboard', true]
	];
</script>

<div
	class="hidden sm:block sticky self-start top-16 lg:top-32 mb-4 w-12 min-w-12 lg:w-46 lg:min-w-46 drop-shadow-sm space-y-2 lg:space-y-4 transition-transform duration-200"
>
	<ul class="space-y-2 bg-white p-2 rounded-xl" id="side-bar">
		{#each routes as [Icon, text, path, routeName, secret], index}
			{#if !secret || authState.isMod}
				<SideBarItem icon={Icon} label={text} href={path} active={routeName === route} />
			{/if}
		{/each}
	</ul>
	<div class="flex flex-col gap-2 bg-white p-1 lg:p-2 rounded-xl">
		<div class="not-lg:w-10 flex flex-col">
			<span class="block not-lg:hidden text-center">Connect with me on:</span>
			<div
				class="flex w-full not-lg:flex-col lg:justify-evenly items-center [&>a]:hover:fill-dark/90"
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
		<div class="not-lg:hidden px-2 flex flex-col font-medium">
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
</style>
