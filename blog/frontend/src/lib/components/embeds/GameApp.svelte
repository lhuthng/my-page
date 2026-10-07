<script>
	/**
	 * `:::app game <slug>` — a playable game embedded in a post or project body.
	 *
	 * The other app embeds each hard-code one launcher (`:::app jsdos`, `:::app
	 * v86`, ...) and resolve a *project* through `/api/projects/s/{slug}`. Games
	 * were split out of projects and carry a launcher type of their own, so this
	 * resolves a game by its public slug instead and lets `ProjectDemo` pick the
	 * player: js-dos, v86, html5/webgl iframe, video or download.
	 *
	 * Only the player frame is rendered — no "Demo" heading and no back link.
	 * Inside an article both are wrong: the heading would join the post's table
	 * of contents (`findHeaders` reads this very DOM), and the article is
	 * already the context the game sits in. The game's own page keeps them.
	 *
	 * The fetch is the public endpoint, so a game that is still a draft (or a
	 * slug typo) reads as a plain "no published game" in the frame rather than a
	 * broken embed.
	 */
	import { onMount } from 'svelte';
	import ProjectDemo from '../project/ProjectDemo.svelte';

	let { name, width = '100%', height = '520px' } = $props();

	let state = $state('loading');
	let errorMessage = $state('');
	let game = $state(null);

	onMount(async () => {
		try {
			const res = await fetch(`/api/games/s/${encodeURIComponent(name)}`);
			if (!res.ok) throw new Error(`No published game with the slug “${name}”.`);
			game = await res.json();
			state = 'ready';
		} catch (error) {
			state = 'error';
			errorMessage = error?.message ?? 'Cannot load this game right now.';
		}
	});
</script>

{#if state === 'ready'}
	<ProjectDemo
		title={game.title ?? name}
		demoType={game.launcher_type ?? 'html5'}
		demoUrl={game.demo_url}
		v86Runtime={game.v86_runtime}
		{width}
		{height}
		showBack={false}
		heading=""
	/>
{:else if state === 'error'}
	<div
		class="mx-auto grid place-items-center rounded-xl bg-background p-4 text-center text-dark/70"
		style:width
		style:height
	>
		<p>{errorMessage}</p>
	</div>
{:else}
	<div
		class="mx-auto grid place-items-center rounded-xl bg-background text-dark/70"
		style:width
		style:height
		role="status"
	>
		Loading game…
	</div>
{/if}
