<script>
	let {
		showMarkdownHelp,
		showKaomojiSearch,
		showGifSearch,
		onToggleHelp,
		onHeader,
		onBold,
		onItalic,
		onCode,
		onToggleKaomoji,
		onToggleGif
	} = $props();
</script>

<div class="comment-editor flex not-sm:flex-col justify-between min-h-8 bg-primary rounded-b-xl">
	<span class="text-white text-base font-semibold px-3 my-auto opacity-80 select-none">
		Markdown Editor
	</span>
	<div
		class="flex flex-wrap *:rounded-xl ml-auto h-full my-auto *:bg-primary fill-white *:w-8 *:h-8 *:*:mx-auto *:hover:brightness-120 *:active:*:translate-y-0.5 gap-2 mr-2"
	>
		<button
			title="Markdown Help"
			type="button"
			class:bg-primary-20={showMarkdownHelp}
			onclick={onToggleHelp}
		>
			<svg class="w-4 h-4" viewBox="0 0 24 24" aria-hidden="true">
				<path
					d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm0 17a1.25 1.25 0 1 1 1.25-1.25A1.25 1.25 0 0 1 12 19Zm1.42-7.6-.57.4a1.48 1.48 0 0 0-.6 1.2v.5h-1.5V13a2.97 2.97 0 0 1 1.23-2.42l.57-.4a1.63 1.63 0 0 0 .7-1.3A1.75 1.75 0 1 0 9.5 9H8a3.25 3.25 0 1 1 6.5-.12 3.14 3.14 0 0 1-1.08 2.52Z"
				></path>
			</svg>
		</button>
		<button title="Header" type="button" onclick={onHeader}>
			<svg class="w-4 h-4" viewBox="0 0 16 16">
				<path
					d="M3.75 2a.75.75 0 0 1 .75.75V7h7V2.75a.75.75 0 0 1 1.5 0v10.5a.75.75 0 0 1-1.5 0V8.5h-7v4.75a.75.75 0 0 1-1.5 0V2.75A.75.75 0 0 1 3.75 2Z"
				></path>
			</svg>
		</button>
		<button title="Bold" type="button" onclick={onBold}>
			<svg class="w-4 h-4" viewBox="0 0 16 16">
				<path
					d="M4 2h4.5a3.501 3.501 0 0 1 2.852 5.53A3.499 3.499 0 0 1 9.5 14H4a1 1 0 0 1-1-1V3a1 1 0 0 1 1-1Zm1 7v3h4.5a1.5 1.5 0 0 0 0-3Zm3.5-2a1.5 1.5 0 0 0 0-3H5v3Z"
				></path>
			</svg>
		</button>
		<button title="Italic" type="button" onclick={onItalic}>
			<svg class="w-4 h-4" viewBox="0 0 16 16">
				<path
					d="M6 2.75A.75.75 0 0 1 6.75 2h6.5a.75.75 0 0 1 0 1.5h-2.505l-3.858 9H9.25a.75.75 0 0 1 0 1.5h-6.5a.75.75 0 0 1 0-1.5h2.505l3.858-9H6.75A.75.75 0 0 1 6 2.75Z"
				></path>
			</svg>
		</button>
		<button title="Code" type="button" onclick={onCode}>
			<svg class="w-4 h-4" viewBox="0 0 16 16">
				<path
					d="m11.28 3.22 4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.749.749 0 0 1-1.275-.326.749.749 0 0 1 .215-.734L13.94 8l-3.72-3.72a.749.749 0 0 1 .326-1.275.749.749 0 0 1 .734.215Zm-6.56 0a.751.751 0 0 1 1.042.018.751.751 0 0 1 .018 1.042L2.06 8l3.72 3.72a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L.47 8.53a.75.75 0 0 1 0-1.06Z"
				></path>
			</svg>
		</button>
		<button
			title="Insert Kaomoji"
			type="button"
			class:bg-primary-20={showKaomojiSearch}
			onclick={onToggleKaomoji}
		>
			<svg class="w-4 h-4" viewBox="0 0 24 24" aria-hidden="true">
				<path
					d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm-3 7.5A1.5 1.5 0 1 1 10.5 8 1.5 1.5 0 0 1 9 9.5Zm6 0A1.5 1.5 0 1 1 16.5 8 1.5 1.5 0 0 1 15 9.5Zm-3 7c-2.13 0-3.77-1.01-4.5-2.78a.75.75 0 1 1 1.39-.57C9.4 14.42 10.5 15 12 15s2.6-.58 3.11-1.85a.75.75 0 0 1 1.39.57C15.77 15.49 14.13 16.5 12 16.5Z"
				></path>
			</svg>
		</button>
		<button
			title="Insert GIF"
			type="button"
			class:bg-primary-20={showGifSearch}
			onclick={onToggleGif}
		>
			<svg class="w-4 h-4" viewBox="0 0 40 40">
				<path
					d="M28.75,11.88V8.94H25.53V6H8.73V34H31.27V11.88Zm-16.94,19V9.08H23v5.46h5.18V30.92Z"
				/>
			</svg>
		</button>
	</div>
</div>

<style lang="postcss">
	@reference "../../../../app.css";

	.comment-editor {
		@apply relative *:relative;
		&::before {
			@apply pointer-events-none absolute! top-0 left-0 z-10 h-full w-full rounded-b-xl border-2 border-primary content-[''];
		}
	}
</style>
