import { browser } from '$app/environment';
import { authState } from '$lib/auth/user.svelte.js';
import { fixUrl } from '$lib/utils/media-path.js';

const POLL_INTERVAL_MS = 60000;
const channel = browser ? new BroadcastChannel('notifications') : null;

function withDisplayAvatar(item) {
	const avatar = item.actor?.avatar_url;
	if (!avatar) return item;
	return { ...item, actor: { ...item.actor, avatar_url: fixUrl(avatar) } };
}

class NotificationStore {
	unread = $state(0);
	items = $state([]);
	hasMore = $state(false);
	loading = $state(false);
	loaded = $state(false);

	reset() {
		this.unread = 0;
		this.items = [];
		this.hasMore = false;
		this.loaded = false;
	}

	async request(path, options = {}) {
		const res = await fetch(`/api/${path}`, {
			...options,
			headers: {
				Authorization: authState.auth(),
				...(options.headers ?? {})
			}
		});
		if (res.status === 401) {
			this.reset();
			return null;
		}
		if (!res.ok) {
			throw new Error(await res.text());
		}
		if (res.status === 204) return null;
		const text = await res.text();
		return text ? JSON.parse(text) : null;
	}

	async refreshCount() {
		if (!authState.user) {
			this.reset();
			return;
		}
		try {
			const data = await this.request('notifications/unread-count');
			if (!data || data.unread === this.unread) return;
			this.unread = data.unread;
			this.broadcast();
		} catch {
			// A transient failure must not break navigation.
		}
	}

	broadcast() {
		channel?.postMessage('refresh');
	}

	async load({ append = false } = {}) {
		if (!authState.user || this.loading) return;
		this.loading = true;
		const before = append && this.items.length > 0 ? this.items[this.items.length - 1].id : null;
		const query = new URLSearchParams({ limit: '20' });
		if (before) query.set('before', String(before));
		try {
			const data = await this.request(`notifications?${query}`);
			if (!data) return;
			this.items = (append ? [...this.items, ...data.items] : data.items).map(withDisplayAvatar);
			this.hasMore = data.has_more;
			this.loaded = true;
		} catch {
			// Leave the previous page in place.
		} finally {
			this.loading = false;
		}
	}

	async reload() {
		await Promise.all([this.load(), this.refreshCount()]);
	}

	async markRead(id) {
		const item = this.items.find((notif) => notif.id === id);
		if (!item || item.read_at) return;
		try {
			await this.request(`notifications/${id}/read`, { method: 'POST' });
		} catch {
			return;
		}
		item.read_at = new Date().toISOString();
		this.unread = Math.max(0, this.unread - 1);
		this.broadcast();
	}

	async markAllRead() {
		try {
			await this.request('notifications/read-all', { method: 'POST' });
		} catch {
			return;
		}
		const now = new Date().toISOString();
		this.items = this.items.map((item) => (item.read_at ? item : { ...item, read_at: now }));
		this.unread = 0;
		this.broadcast();
	}
}

export const notificationState = new NotificationStore();

if (channel) {
	channel.onmessage = () => {
		notificationState.refreshCount();
		if (notificationState.loaded) notificationState.load();
	};
}

export function notificationHref(item) {
	const section = { post: 'posts', project: 'projects', game: 'games' }[item.post.kind] ?? 'posts';
	return `/${section}/${item.post.slug}#comment-${item.comment.id}`;
}

export function notificationLabel(item) {
	switch (item.kind) {
		case 'reply':
			return 'replied to your comment';
		case 'mention':
			return 'mentioned you';
		default:
			return 'commented on your post';
	}
}

let pollTimer;
let polling = false;

function poll() {
	if (document.visibilityState === 'visible') {
		notificationState.refreshCount();
	}
}

export function startNotificationPolling() {
	if (polling) return;
	polling = true;
	notificationState.refreshCount();
	pollTimer = setInterval(poll, POLL_INTERVAL_MS);
	document.addEventListener('visibilitychange', poll);
	window.addEventListener('focus', poll);
}

export function stopNotificationPolling() {
	if (!polling) return;
	polling = false;
	clearInterval(pollTimer);
	document.removeEventListener('visibilitychange', poll);
	window.removeEventListener('focus', poll);
}
