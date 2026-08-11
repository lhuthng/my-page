function startPlayback(node) {
	const attemptPlay = () => node.play?.().catch(() => {});

	if (node.readyState >= 1) {
		attemptPlay();
		return;
	}

	if (node.src && !node.currentSrc) node.load();
	node.addEventListener('loadedmetadata', attemptPlay, { once: true });
	attemptPlay();
}

export function lazyVideo(node) {
	if (typeof IntersectionObserver === 'undefined') {
		node.autoplay = true;
		return {};
	}

	const observer = new IntersectionObserver(
		(entries) => {
			for (const entry of entries) {
				if (entry.isIntersecting) {
					if (!node.src && node.dataset.src) node.src = node.dataset.src;
					startPlayback(node);
				} else {
					node.pause?.();
				}
			}
		},
		{ rootMargin: '200px' }
	);

	observer.observe(node);

	return {
		destroy() {
			observer.disconnect();
		}
	};
}
