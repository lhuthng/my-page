<script>
	import PBody from '../shell/PBody.svelte';
	import PostSection from '../post/PostSection.svelte';

	// Renders the exact public post/project layout (PostSection) in an overlay,
	// so the editor's split pane only has to host the lightweight
	// title + content preview.
	let {
		show,
		id = null,
		title,
		tags = [],
		date = '',
		content,
		relatedPosts = [],
		onclose
	} = $props();
</script>

{#if show}
	<PBody>
		<div class="fixed z-5 flex w-screen h-screen items-center justify-center">
			<button
				class="absolute full cursor-default! bg-dark/40"
				onclick={onclose}
				title="close-overlay"
			></button>
			<div
				class="relative z-10 flex flex-col gap-4 w-full mt-13 lg:mt-28 mx-2 max-h-[calc(100dvh-5rem)] lg:max-h-[calc(100dvh-8.25rem)] max-w-7xl overflow-hidden rounded-xl bg-white/95 p-4 text-dark"
				role="none"
			>
				<div class="flex items-center justify-between shrink-0">
					<h2 class="text-lg font-semibold">Full preview</h2>
					<button
						class="flex h-8 w-8 items-center justify-center rounded-lg border border-background bg-background/40 text-lg leading-none text-dark transition-colors hover:bg-background/60"
						onclick={onclose}
						title="Close preview"
					>
						✕
					</button>
				</div>
				<div class="min-w-0 overflow-y-auto custom-scrollbar">
					<PostSection {id} {title} {tags} {date} {content} {relatedPosts} hideBackButton />
				</div>
			</div>
		</div>
	</PBody>
{/if}
