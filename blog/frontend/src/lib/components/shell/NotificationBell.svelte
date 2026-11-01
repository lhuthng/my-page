<script>
	import { goto } from '$app/navigation';
	import { authState } from '$lib/auth/user.svelte.js';
	import {
		notificationHref,
		notificationLabel,
		notificationState
	} from '$lib/notifications/store.svelte.js';
	import { relativeTime } from '$lib/notifications/time.js';
	import { fly } from 'svelte/transition';
	import { innerHeight } from 'svelte/reactivity/window';
	import { win } from '$lib/dom/windows.svelte.js';

	// A preview row is ~76px, the header + footer chrome is 12rem, and the list
	// is capped at 24rem from lg up, so both limits are already in the layout.
	const ROW_PX = 76;
	const CHROME_PX = 192;
	const DESKTOP_BODY_PX = 384;
	const MAX_ITEMS = 5;

	let open = $state(false);

	let unread = $derived(notificationState.unread);
	let badge = $derived(unread > 99 ? '99+' : String(unread));
	let maxItems = $derived.by(() => {
		const viewport = innerHeight.current ?? 800;
		const body = win.isLg ? DESKTOP_BODY_PX : viewport - CHROME_PX;
		return Math.min(MAX_ITEMS, Math.max(2, Math.floor(body / ROW_PX)));
	});

	async function toggle() {
		open = !open;
		if (open) {
			await notificationState.load();
			// Opening the panel shows the newest page; the count stays truthful
			// until an item is clicked or "Mark all" runs.
			await notificationState.refreshCount();
		}
	}

	async function openItem(item) {
		open = false;
		await notificationState.markRead(item.id);
		await goto(notificationHref(item));
	}
</script>

{#if authState.user}
	<div class="relative">
		<button
			type="button"
			aria-label={unread > 0 ? `Notifications (${unread} unread)` : 'Notifications'}
			aria-expanded={open}
			class="relative rounded-full p-1.5 text-dark transition-colors hover:bg-primary/20"
			onclick={toggle}
		>
			<svg
				class="h-6 w-6 lg:h-7 lg:w-7"
				viewBox="0 0 24 24"
				fill={open ? 'currentColor' : 'none'}
				stroke="currentColor"
				stroke-width="2"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					d="M15 17h5l-1.4-1.4A2 2 0 0 1 18 14.2V11a6 6 0 1 0-12 0v3.2c0 .5-.2 1-.6 1.4L4 17h5m6 0a3 3 0 1 1-6 0"
				/>
			</svg>
			{#if unread > 0}
				<span
					class="absolute -right-0.5 -top-0.5 flex h-5 min-w-5 items-center justify-center rounded-full bg-accent-red px-1 text-sm font-bold text-white"
				>
					{badge}
				</span>
			{/if}
		</button>

		{#if open}
			<button
				type="button"
				class="fixed inset-0 z-40 cursor-default"
				aria-label="Close notifications"
				onclick={() => (open = false)}
			></button>
			<div
				in:fly={{ y: -8, duration: 150 }}
				class="fixed top-[var(--content-top)] right-2 left-2 z-50 overflow-hidden rounded-xl border-2 border-dark bg-white shadow-xl sm:left-auto sm:w-[24rem] lg:absolute lg:top-full lg:right-0 lg:mt-2 lg:w-[22rem]"
			>
				<div class="flex items-center justify-between border-b-2 border-dark/10 px-3 py-2">
					<span class="font-bold">Notifications</span>
					<button
						type="button"
						class="text-sm text-primary hover:underline disabled:opacity-40"
						disabled={unread === 0}
						onclick={() => notificationState.markAllRead()}
					>
						Mark all as read
					</button>
				</div>

				<div class="max-h-[calc(100dvh-12rem)] overflow-y-auto custom-scrollbar lg:max-h-96">
					{#if notificationState.loading && notificationState.items.length === 0}
						<p class="px-3 py-6 text-center text-sm text-dark/50">Loading...</p>
					{:else if notificationState.items.length === 0}
						<p class="px-3 py-6 text-center text-sm text-dark/50">Nothing yet.</p>
					{:else}
						<ul>
							{#each notificationState.items.slice(0, maxItems) as item (item.id)}
								<li>
									<button
										type="button"
										class="flex w-full items-start gap-3 rounded-lg px-3 py-2 text-left transition-colors {!item.read_at
											? 'bg-role-surface hover:bg-notification-hover-unread'
											: 'hover:bg-notification-hover'}"
										onclick={() => openItem(item)}
									>
										<img
											class="mt-0.5 h-9 w-9 shrink-0 rounded-full border-2 object-cover"
											src={item.actor.avatar_url ?? '/missing.png'}
											alt=""
										/>
										<span class="min-w-0 grow">
											<span class="block text-sm leading-snug">
												<strong>{item.actor.display_name ?? 'Someone'}</strong>
												{notificationLabel(item)}
											</span>
											<span class="block truncate text-sm text-dark/60">{item.post.title}</span>
											<span class="block text-sm text-dark/40">
												{relativeTime(item.created_at)}
											</span>
										</span>
										{#if !item.read_at}
											<span class="mt-1.5 h-2 w-2 shrink-0 rounded-full bg-primary"></span>
										{/if}
									</button>
								</li>
							{/each}
						</ul>
					{/if}
				</div>

				<a
					href="/notifications"
					class="block border-t-2 border-dark/10 px-3 py-2 text-center text-sm font-semibold text-primary hover:bg-primary/10"
					onclick={() => (open = false)}
				>
					See all
				</a>
			</div>
		{/if}
	</div>
{/if}
