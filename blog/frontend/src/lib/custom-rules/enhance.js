import App from '$lib/components/embeds/App.svelte';
import { mount, unmount } from 'svelte';
import ImagePreviewer from '$lib/components/embeds/ImagePreviewer.svelte';
import { el } from '$lib/dom/elements.svelte.js';

/**
 * Mount interactive behaviour into a rendered markdown tree, and return a
 * single teardown function that undoes all of it.
 *
 * This is called by `Post.svelte`'s `{@attach}`, which re-runs it whenever the
 * rendered HTML changes and always calls the previous run's cleanup first.
 * That means every debounce tick in the editor preview re-mounts this tree —
 * so a cleanup that only tore down `App` instances (as this used to) leaked a
 * `reveal`/`audio-sync`/`expandable-image` listener set, plus one injected
 * "Sync Time" button, on every keystroke-driven re-render.
 */
export function pluginExtend(root) {
	const appInstances = new Map();
	const teardowns = [];

	const appContainers = root.querySelectorAll('.app-container');
	appContainers.forEach((container) => {
		if (container.__mounted) return;
		container.__mounted = true;

		const { name, type, width, height, config, temp } = container.dataset;

		appInstances.set(
			container,
			mount(App, {
				target: container,
				props: { name, type, width, height, config, temp }
			})
		);
	});

	const revealContainers = root.querySelectorAll('.reveal');
	revealContainers.forEach((container) => {
		if (container.__mounted) return;
		container.__mounted = true;

		const button = container.querySelector('.reveal-tooltip');
		const originalText = button.textContent;

		const onClick = () => {
			container.classList.toggle('toggled');
			const isToggled = container.classList.contains('toggled');
			button.textContent = isToggled ? 'Click to hide' : originalText;
		};
		button.addEventListener('click', onClick);
		teardowns.push(() => button.removeEventListener('click', onClick));
	});

	const audioSyncContainers = root.querySelectorAll('.audio-sync-container');
	audioSyncContainers.forEach((container) => {
		if (container.__mounted) return;
		container.__mounted = true;

		const audios = container.querySelectorAll('.audio-container audio');
		let isSyncing = false;

		const syncPlay = () => {
			audios.forEach((audio) => {
				audio.play();
			});
		};

		const syncPause = () => {
			if (isSyncing) return;
			audios.forEach((audio) => {
				audio.pause();
			});
		};

		const syncTime = (time) => {
			audios.forEach((audio) => {
				audio.currentTime = time;
			});
		};

		audios.forEach((audio) => {
			audio.addEventListener('play', syncPlay);
			audio.addEventListener('pause', syncPause);
		});
		teardowns.push(() => {
			audios.forEach((audio) => {
				audio.removeEventListener('play', syncPlay);
				audio.removeEventListener('pause', syncPause);
			});
		});

		const duoBtn = document.createElement('div');
		duoBtn.className = 'mx-auto w-fit duo-btn';
		duoBtn.dataset.duoColor = 'dark';
		const btn = document.createElement('button');
		btn.textContent = 'Sync Time';
		duoBtn.append(btn);
		container.appendChild(duoBtn);
		const onSyncClick = () => {
			let avg = 0;
			audios.forEach((audio) => (avg += audio.currentTime / audios.length));
			syncTime(avg);
		};
		btn.addEventListener('click', onSyncClick);
		teardowns.push(() => {
			btn.removeEventListener('click', onSyncClick);
			duoBtn.remove();
		});
	});

	const expandableImageContainers = root.querySelectorAll('img.expandable');
	expandableImageContainers.forEach((container) => {
		if (container.__mounted) return;
		container.__mounted = true;

		// Tracked so the teardown can unmount an still-open previewer instead of
		// leaking it if the underlying markdown re-renders while it's open.
		let openPreviewer = null;
		let opened = false;
		const onClick = () => {
			if (opened) return;
			opened = true;

			openPreviewer = mount(ImagePreviewer, {
				target: el.mbody,
				props: {
					visible: true,
					origin: container,
					onClose: () => {
						unmount(openPreviewer, { outro: true });
						openPreviewer = null;
						opened = false;
					}
				}
			});
		};
		container.addEventListener('click', onClick);
		teardowns.push(() => {
			container.removeEventListener('click', onClick);
			if (openPreviewer) {
				unmount(openPreviewer, { outro: true });
				openPreviewer = null;
			}
		});
	});

	return () => {
		for (const [container, app] of appInstances) {
			unmount(app);
			container.__mounted = false;
		}
		appInstances.clear();

		for (const teardown of teardowns) teardown();
		teardowns.length = 0;
	};
}
