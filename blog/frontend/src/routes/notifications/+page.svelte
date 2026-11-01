<script>
	import { authState } from '$lib/auth/user.svelte.js';
	import {
		notificationHref,
		notificationLabel,
		notificationState
	} from '$lib/notifications/store.svelte.js';
	import { relativeTime } from '$lib/notifications/time.js';
	import BackButton from '$lib/components/ui/BackButton.svelte';
	import { fly } from 'svelte/transition';

	let filter = $state('all');

	let items = $derived(
		filter === 'unread'
			? notificationState.items.filter((item) => !item.read_at)
			: notificationState.items
	);

	// The layout hydrates authState from server data in an effect, so wait for
	// the user before the first authenticated request.
	$effect(() => {
		if (authState.user && !notificationState.loaded && !notificationState.loading) {
			notificationState.reload();
		}
	});
</script>

<svelte:head>
	<title>Notifications</title>
</svelte:head>

<div class="bg-white rounded-xl mb-2 lg:mb-4">
	<div class="px-4 pt-4 space-y-2">
		<BackButton href="/" text="Home" />
		<div class="flex flex-wrap items-center justify-between gap-2 lg:gap-4">
			<h1 class="text-2xl font-semibold">Notifications</h1>
			<div class="flex items-center gap-2">
				<div
					class="flex rounded-lg border-2 border-dark bg-dark text-sm transition-transform duration-50 {notificationState.unread ===
					0
						? 'translate-y-[0.2rem]'
						: ''}"
				>
					<button
						type="button"
						class="rounded-l-[6px] px-3 py-1 {filter === 'all' ? 'bg-dark text-white' : 'bg-white'}"
						onclick={() => (filter = 'all')}
					>
						All
					</button>
					<button
						type="button"
						class="rounded-r-[6px] border-l-2 border-dark px-3 py-1 {filter === 'unread'
							? 'bg-dark text-white'
							: 'bg-white'}"
						onclick={() => (filter = 'unread')}
					>
						Unread
					</button>
				</div>
				<div class="duo-btn" data-duo-color="primary">
					<button
						type="button"
						disabled={notificationState.unread === 0}
						onclick={() => notificationState.markAllRead()}
					>
						Mark all as read
					</button>
				</div>
			</div>
		</div>
	</div>

	<div class="p-4">
		{#if !notificationState.loaded && notificationState.loading}
			<p class="py-10 text-center text-dark/50">Loading...</p>
		{:else if items.length === 0}
			<p class="py-10 text-center text-dark/50">
				{filter === 'unread' ? 'No unread notifications.' : 'Nothing yet.'}
			</p>
		{:else}
			<ul class="flex flex-col gap-2 lg:gap-4">
				{#each items as item (item.id)}
					<li in:fly={{ y: 6, duration: 120 }}>
						<a
							href={notificationHref(item)}
							class="flex items-start gap-3 rounded-xl border-2 border-dark/10 bg-role-surface p-4 text-role-text no-underline! transition-colors hover:bg-notification-hover-unread"
							onclick={() => notificationState.markRead(item.id)}
						>
							<img
								class="mt-0.5 h-11 w-11 shrink-0 rounded-full border-2 object-cover"
								src={item.actor.avatar_url ?? '/missing.png'}
								alt=""
							/>
							<span class="min-w-0 grow">
								<span class="block">
									<strong>{item.actor.display_name ?? 'Someone'}</strong>
									{notificationLabel(item)}
								</span>
								<span class="block truncate text-sm text-role-text">{item.post.title}</span>
								<span class="mt-1 block text-sm italic text-dark/50">
									{relativeTime(item.created_at)}
								</span>
								<span class="mt-2 block truncate text-sm text-dark/60">
									{#each item.comment.preview as part, index (index)}{#if part.kind}<span
												class="italic text-role-muted"
											>
												{part.text}
											</span>{:else}{part.text}{/if}{/each}
								</span>
							</span>
							{#if !item.read_at}
								<span class="mt-2 h-2.5 w-2.5 shrink-0 rounded-full bg-primary"></span>
							{/if}
						</a>
					</li>
				{/each}
			</ul>

			{#if notificationState.hasMore}
				<div class="mx-auto mt-2 w-48 duo-btn lg:mt-4" data-duo-color="primary">
					<button
						type="button"
						disabled={notificationState.loading}
						onclick={() => notificationState.load({ append: true })}
					>
						{notificationState.loading ? 'Loading...' : 'Load more'}
					</button>
				</div>
			{/if}
		{/if}
	</div>
</div>
