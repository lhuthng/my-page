import { api } from './api-client';

export function usePagination({
	url,
	limit,
	selectItems = (data) => data.items ?? [],
	selectHasMore,
	initialItems = [],
	initialHasMore = true,
	useAuth = false
} = {}) {
	let items = $state(initialItems);
	let hasMore = $state(selectHasMore ? selectHasMore({ items: initialItems }) : initialHasMore);
	let loading = $state(false);
	let error = $state('');

	const fetchMore = async () => {
		if (loading || !hasMore) return;
		loading = true;
		error = '';
		try {
			const data = await api.get(`${url}&offset=${items.length}`, { auth: useAuth });
			const newItems = selectItems(data);
			items = [...items, ...newItems];
			if (selectHasMore) {
				hasMore = selectHasMore(data);
			} else {
				hasMore = newItems.length >= limit;
			}
		} catch (e) {
			error = e.message || 'Failed to load more.';
		} finally {
			loading = false;
		}
	};

	const reset = (newInitialItems = []) => {
		items = newInitialItems;
		hasMore = initialHasMore;
		loading = false;
		error = '';
	};

	return {
		get items() {
			return items;
		},
		get hasMore() {
			return hasMore;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},
		fetchMore,
		reset
	};
}
