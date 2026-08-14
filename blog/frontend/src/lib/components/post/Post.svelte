<script>
	import App from '../embeds/App.svelte';
	import { findHeaders, pluginExtend } from '$lib/custom-rules';

	let { content, headers = $bindable() } = $props();

	function applyPlugins(e) {
		content; // To be reactive
		const cleanup = pluginExtend(e);
		headers = findHeaders(e);
		return cleanup;
	}
</script>

<div class="rendered-markdown" {@attach applyPlugins}>
	{@html content}
</div>

<style lang="postcss">
	@reference "../../../app.css";

	:global(.rendered-markdown) {
		& {
			@apply space-y-1;
			counter-reset: h1;
		}

		& hr {
			@apply mt-4 border-t-2 border-dashed;
		}
		& a {
			@apply mr-2 text-accent-blue;
		}
		& a::after {
			content: '^';
			@apply absolute inline-block -rotate-30 align-middle text-sm;
		}
		& h1,
		& h2,
		& h3 {
			@apply scroll-mt-16 lg:scroll-mt-30;
		}
		& h1 {
			@apply mt-6 text-2xl font-bold;
			counter-increment: h1;
			counter-reset: h2;
		}
		& h1::before {
			@apply font-bold;
			content: counter(h1) '. ';
		}
		& h2 {
			@apply mt-4 text-xl font-bold;
			counter-reset: h3;
		}
		& h2::before {
			@apply font-bold;
			counter-increment: h2;
			content: counter(h1) '.' counter(h2) '. ';
		}
		& h3 {
			@apply mt-1 text-lg font-semibold;
			counter-reset: h4;
		}
		& h3::before {
			@apply font-bold;
			counter-increment: h3;
			content: counter(h1) '.' counter(h2) '.' counter(h3) '. ';
		}
		& img {
			@apply mx-auto rounded-lg;
		}
		& img.expandable {
			@apply cursor-pointer;
		}
		& p {
			@apply mt-4;
		}
		& ul {
			@apply list-disc px-4;
		}
		& li {
			@apply mt-1;
		}
		& li > ul,
		& li > ol {
			@apply ml-1;
		}
		& ol {
			@apply list-inside list-decimal;
		}
		& table {
			@apply mx-auto my-6 border-2 border-dark;
		}
		& thead {
			@apply border-2 border-dark bg-dark/90 text-white;
		}
		& tr {
			@apply text-left;
		}
		& td {
			@apply p-1 not-first:border-l-2;
		}
		& th {
			@apply border-dark p-1 not-first:border-l-2;
		}
		& p > code,
		& li > code {
			@apply rounded-md bg-gray-200 p-1 hover:brightness-95;
		}
		& blockquote {
			@apply relative ml-4 pl-4;
		}
		& blockquote::after {
			content: '';
			@apply absolute top-0 left-0 h-full w-0.5 bg-dark;
		}
		& code {
			@apply inline break-after-all wrap-break-word whitespace-pre-wrap not-sm:text-sm;
		}
		& .reveal {
			@apply mt-4 flex max-h-11 flex-col overflow-x-hidden overflow-y-hidden rounded-xl border-2 bg-dark p-2 text-white;
		}
		& .reveal > .reveal-content {
			@apply pointer-events-none -translate-y-4 opacity-0 transition-all duration-200;
		}
		& .reveal > .reveal-tooltip {
			@apply w-full;
		}
		& .reveal p > code,
		& .reveal li > code {
			@apply bg-white/15 text-white hover:bg-white/25;
		}
		& .reveal pre {
			@apply border border-white/20 bg-white/10;
		}
		& .reveal.toggled {
			@apply max-h-full;
		}

		& .reveal.toggled > .reveal-content {
			@apply pointer-events-auto translate-y-0 opacity-100;
		}

		& .katex-display {
			@apply overflow-x-auto overflow-y-hidden;
			scrollbar-gutter: stable;
			scrollbar-width: thin;
			scrollbar-color: rgba(73, 92, 131, 0.8) transparent;
		}
		& .katex-display::-webkit-scrollbar-thumb {
			background-color: rgba(73, 92, 131, 0.8);
			border-radius: 10px;
			border: 2px solid transparent;
			background-clip: content-box;
		}
		& .katex-display::-webkit-scrollbar-thumb:hover {
			background-color: rgba(73, 92, 131, 0.9);
		}
		& .katex-display::-webkit-scrollbar {
			width: 10px;
		}
		& .katex-display::-webkit-scrollbar-track {
			background: transparent;
		}
		& .reveal .katex-display {
			scrollbar-color: rgba(255 255, 255, 0.8) transparent;
		}
		& .reveal .katex-display::-webkit-scrollbar-thumb {
			background-color: rgba(255, 255, 255, 0.8);
		}
		& .reveal .katex-display::-webkit-scrollbar-thumb:hover {
			background-color: rgba(255, 255, 255, 0.9);
		}
		& .katex-display > .katex {
			@apply whitespace-nowrap;
		}
		& .audio-container {
			@apply w-full py-2;
		}
		& .audio-container > audio {
			@apply mx-auto rounded-full border-2 border-secondary;
		}
		& .video-container {
			@apply w-full py-4;
		}
		& .video-container > video {
			@apply mx-auto overflow-hidden rounded-lg;
		}
		& .audio-sync-container {
			@apply mx-auto w-fit rounded-[2.8rem] border-2 border-secondary bg-secondary px-5 pb-4;
		}
		& span.kaomoji {
			display: inline-block;
			width: max-content;
			max-width: 100%;
			white-space: normal;
		}
	}
</style>
