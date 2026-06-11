<script>
	import PostCard from './PostCard.svelte';
	import gsap from 'gsap';
	import { Flip } from 'gsap/Flip';
	import { onMount, tick, untrack } from 'svelte';
	import { fly } from 'svelte/transition';
	import { flip } from 'svelte/animate';

	const { featuredPosts } = $props();

	const itemDelay = 45;

	let hydrated = $state(false);

	onMount(() => {
		hydrated = true;
	});

	let categories = $state(
		['programming', 'art', 'music'].map((tag) => ({
			value: tag,
			selected: false
		}))
	);

	let tabContainer = $state();
	let discoverTab = $state();
	let freshTab = $state();
	let skipIntro = $state(false);

	let tab = $state({
		preindex: 1,
		index: 1,
		container: null,
		discover: null,
		fresh: null
	});

	let fresh = $state({
		created: {
			status: 'unset',
			cache: []
		},
		updated: {
			status: 'unset',
			cache: []
		}
	});

	$effect(() => {
		if (!tab.container || !tab.discover || !tab.fresh) return;

		if (tab.preindex === tab.index) return;

		tab.preindex = tab.index;

		const state = Flip.getState(tab.container);

		if (tab.index !== 1) {
			tab.discover.classList.toggle('hidden', true);
			tab.fresh.classList.toggle('hidden', false);
			tab.fresh.classList.toggle('grid', true);
		} else {
			tab.discover.classList.toggle('hidden', false);
			tab.fresh.classList.toggle('hidden', true);
			tab.fresh.classList.toggle('grid', false);
		}

		Flip.from(state, { duration: 0.5, ease: 'power3.inOut' });
	});

	let order = $state();
	let _order = $state();

	$effect(() => {
		if (tab.index !== 2) {
			if (order === undefined) {
				order = 'created';
			}

			_order = order;
		}
	});

	$effect(async () => {
		if (tab.index !== 2) return;

		order;

		const orderFresh = untrack(() => fresh[order]);

		if (orderFresh.status === 'fetched' || orderFresh.status === 'pending') return;

		orderFresh.status = 'pending';

		const res = await fetch(
			`api/posts/latest?limit=5&offset=0&${
				order === 'created' ? 'sorted_by_created=true' : 'sorted_by_updated=true'
			}`,
			{
				method: 'GET'
			}
		);

		if (res.ok) {
			setTimeout(async () => {
				const state = Flip.getState(tab.container);
				const payload = await res.json();

				orderFresh.cache = (payload.featured_posts ?? []).map((post, index) => ({
					...post,
					_introDelay: index * itemDelay
				}));

				orderFresh.status = 'fetched';

				await tick();

				Flip.from(state, { duration: 0.5, ease: 'power3.inOut' });
			}, 500);
		} else {
			orderFresh.cache = [];
			orderFresh.status = 'failed';
		}
	});
</script>

{#snippet exploreMore(link)}
	<li
		class="flex justify-center items-center full min-w-22 sm:min-w-26 min-h-22 sm:min-h-26 md:min-w-34 md:min-h-34 rounded-lg border-2 border-dashed"
	>
		<div class="duo-btn duo-blue">
			<a class="no-underline!" href={link}>explore more</a>
		</div>
	</li>
{/snippet}

<div class="space-y-4 pt-4">
	<div class="flex not-sm:flex-col min-h-10 items-center justify-between">
		<ul id="home-tab" class="text-lg sm:text-xl font-medium h-8">
			<li class:left={true} class:selected={tab.index === 1}>
				<button
					onclick={() => {
						tab.index = 1;
						skipIntro = false;
					}}
				>
					Discover
				</button>
			</li>
			<li class:right={true} class:selected={tab.index === 2}>
				<button
					onclick={() => {
						tab.index = 2;
						skipIntro = false;
					}}
				>
					Fresh
				</button>
			</li>
		</ul>

		{#if tab.index === 2}
			<select
				class="focus:outline-none border-2 border-dark p-1 rounded-lg hover:bg-dark hover:text-white transition-colors duration-200 not-sm:mt-2 sm:ml-auto w-fit cursor-pointer text-base sm:text-lg"
				name="post-filter"
				bind:value={_order}
				in:fly={{ y: -10, duration: 200 }}
				onchange={async (e) => {
					const value = e.target.value;

					if (fresh[value].status !== 'fetched') {
						const state = Flip.getState(tab.container);
						order = value;
						await tick();
						Flip.from(state, { duration: 0.5, ease: 'power3.inOut' });
					} else {
						order = value;
					}

					skipIntro = fresh?.[order]?.status === 'fetched';
					_order = order;
				}}
			>
				<option value="created">By Created</option>
				<option value="updated">By Updated</option>
			</select>
		{/if}
	</div>

	<div bind:this={tab.container} class="pb-2">
		<ul
			bind:this={tab.discover}
			class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4"
		>
			{#each featuredPosts as { title, slug, excerpt, author_name, author_slug, tag_slugs, url, stats }, index (slug)}
				<li class:intro={hydrated} style:--delay={`${index * itemDelay}ms`}>
					<PostCard
						{title}
						{slug}
						{excerpt}
						author={{
							name: author_name,
							slug: author_slug
						}}
						tags={tag_slugs}
						src={url}
						{stats}
					/>
				</li>
			{/each}

			<li
				class="flex justify-center items-center full min-w-22 sm:min-w-26 min-h-22 sm:min-h-26 md:min-w-34 md:min-h-34 rounded-lg border-2 border-dashed"
				class:intro={hydrated}
				style:--delay={`${featuredPosts.length * itemDelay}ms`}
			>
				<div class="duo-btn duo-blue">
					<a class="no-underline!" href="/posts">explore more</a>
				</div>
			</li>
		</ul>

		<ul
			bind:this={tab.fresh}
			class="hidden grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4"
		>
			{#if fresh[order]?.status === 'fetched'}
				{#each fresh[order].cache as post (post.slug)}
					<li
						class:intro={hydrated && !skipIntro}
						style:--delay={`${post._introDelay}ms`}
						animate:flip={{ delay: post._introDelay, duration: 500 }}
					>
						<PostCard
							title={post.title}
							slug={post.slug}
							excerpt={post.excerpt}
							author={{
								name: post.author_name,
								slug: post.author_slug
							}}
							tags={post.tag_slugs}
							src={post.url}
							stats={post.stats}
						/>
					</li>
				{/each}

				<li
					class="flex justify-center items-center full min-w-22 sm:min-w-26 min-h-22 sm:min-h-26 md:min-w-34 md:min-h-34 rounded-lg border-2 border-dashed"
					class:intro={hydrated}
					style:--delay={`${fresh[order].cache.length * itemDelay}ms`}
				>
					<div class="duo-btn duo-blue">
						<a class="no-underline!" href="/posts">explore more</a>
					</div>
				</li>
			{:else}
				<div class="w-full col-span-full py-10 text-center">Loading</div>
			{/if}
		</ul>
	</div>
</div>

<style lang="postcss">
	@reference "../../../app.css";

	.intro {
		animation: fly-in 420ms ease-out both;
		animation-delay: var(--delay);
	}

	@keyframes fly-in {
		from {
			opacity: 0;
			transform: translateY(-20px);
		}

		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	#home-tab {
		@apply flex gap-4;

		li {
			@apply relative;

			button {
				@apply relative z-10 text-dark/70;
			}

			&::before {
				@apply absolute -top-1 z-9 h-[calc(100%+0.25rem)] w-0 bg-linear-to-t from-background/40 via-background/20 to-primary/0 transition-all duration-200 content-[""];
			}

			&::after {
				@apply absolute bottom-0 z-9 h-1 w-0 bg-dark transition-all duration-200 content-[""];
			}

			&.left::after,
			&.left::before {
				right: -0.5rem;
			}

			&.right::after,
			&.right::before {
				left: -0.5rem;
			}
		}

		li.selected > button {
			@apply relative z-10 text-dark;
		}

		li.selected::after,
		li.selected::before {
			@apply w-[calc(100%+1rem)];
		}
	}

	button {
		@apply focus:outline-none;
	}
</style>
