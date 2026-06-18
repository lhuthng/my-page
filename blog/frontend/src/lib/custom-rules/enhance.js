import App from '$lib/components/embeds/App.svelte';
import { mount, unmount } from 'svelte';
import ImagePreviewer from '$lib/components/embeds/ImagePreviewer.svelte';
import { el } from '$lib/dom/elements.svelte.js';

export function pluginExtend(root) {
	const appContainers = root.querySelectorAll('.app-container');
	appContainers.forEach((container) => {
		if (container.__mounted) return;
		container.__mounted = true;

		const { name, type, width, height, config, temp } = container.dataset;

		mount(App, {
			target: container,
			props: { name, type, width, height, config, temp }
		});
	});

	const revealContainers = root.querySelectorAll('.reveal');
	revealContainers.forEach((container) => {
		if (container.__mounted) return;
		container.__mounted = true;

		const button = container.querySelector('.reveal-tooltip');
		const originalText = button.textContent;

		button.addEventListener('click', () => {
			container.classList.toggle('toggled');
			const isToggled = container.classList.contains('toggled');
			button.textContent = isToggled ? 'Click to hide' : originalText;
		});
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

		const duoBtn = document.createElement('div');
		duoBtn.className = 'mx-auto w-fit duo-btn';
		duoBtn.dataset.duoColor = 'dark';
		const btn = document.createElement('button');
		btn.textContent = 'Sync Time';
		duoBtn.append(btn);
		container.appendChild(duoBtn);
		btn.addEventListener('click', () => {
			let avg = 0;
			audios.forEach((audio) => (avg += audio.currentTime / audios.length));
			syncTime(avg);
		});
	});

	const expandableImageContainers = root.querySelectorAll('img.expandable');
	expandableImageContainers.forEach((container) => {
		if (container.__mounted) return;
		container.__mounted = true;

		let opened = false;
		container.addEventListener('click', () => {
			if (opened) return;
			opened = true;

			let previewer = mount(ImagePreviewer, {
				target: el.mbody,
				props: {
					visible: true,
					origin: container,
					onClose: () => {
						unmount(previewer, { outro: true });
						opened = false;
					}
				}
			});
		});
	});
}
