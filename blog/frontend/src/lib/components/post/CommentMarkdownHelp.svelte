<script>
	import { fade } from 'svelte/transition';

	let { open = false, close = () => {} } = $props();
</script>

{#snippet dim(text, className)}
	<span class={className ?? 'opacity-60'}>{text}</span>
{/snippet}

{#if open}
	<div class="fixed inset-0 z-40 max-h-full overflow-y-hidden" role="presentation" transition:fade>
		<div
			class="absolute inset-2 lg:inset-10 max-h-[100%-5rem] overflow-y-auto rounded-lg border-2 border-primary bg-white p-4 shadow-xl custom-scrollbar overscroll-contain not-lg:overscroll-auto"
			role="dialog"
			tabindex="-1"
			aria-modal="true"
			aria-labelledby="markdown-help-title"
		>
			<div class="mb-3 flex items-start justify-between gap-4">
				<div>
					<h5 id="markdown-help-title" class="text-xl font-bold text-dark">Markdown Help</h5>
					<p class="text-sm text-dark/70">Quick syntax reference for comments.</p>
				</div>
				<div class="duo-btn duo-primary">
					<button type="button" class="text-sm font-semibold" onclick={close}>Close</button>
				</div>
			</div>

			<div class="space-y-4 text-sm text-dark/90">
				<div>
					<p class="mb-1 font-semibold">Basic Markdown</p>
					<ul class="space-y-1 list-disc pl-5">
						<li>
							<code># {@render dim('Heading')}</code>
							for a title line
						</li>
						<li>
							<code>**{@render dim('bold text')}**</code>
							for bold emphasis
						</li>
						<li>
							<code>_{@render dim('italic text')}_</code>
							for italic emphasis
						</li>
						<li>
							<code>`{@render dim('inline code')}`</code>
							for code snippets
						</li>
						<li>
							<code>
								![{@render dim('alt text')}]({@render dim(
									'https://example.com/image.gif',
									'text-accent-blue'
								)})
							</code>
							for images/GIFs
						</li>
					</ul>
				</div>

				<div>
					<p class="mb-1 font-semibold">Extended Markdown</p>
					<ul class="space-y-1 list-disc pl-5">
						<li>
							<code>
								@@[
								{@render dim('( ꩜ ᯅ ꩜;)')}
								]@@
							</code>
							for anything would stay together like a Kaomoji
						</li>
					</ul>
				</div>

				<div>
					<p class="mb-1 font-semibold">Mention Syntax</p>
					<ul class="space-y-1 list-disc pl-5">
						<li>
							Type <code>@{@render dim('username')}</code>
							(letters, numbers,
							<code>_</code>
							,
							<code>-</code>
							; min 3 chars)
						</li>
						<li>
							Suggestions appear while typing after <code>@</code>
						</li>
						<li>
							Use <code>Arrow Up/Down</code>
							to pick, then
							<code>Enter</code>
							or
							<code>Tab</code>
						</li>
						<li>
							Use <code>Escape</code>
							to close mention suggestions
						</li>
					</ul>
				</div>

				<div>
					<p class="mb-1 font-semibold">Command Syntax</p>
					<ul class="space-y-1 list-disc pl-5">
						<li>
							<code>/kao {@render dim('mood')}</code>
							to search kaomoji by mood (example:
							<code>/kao joy</code>
							)
						</li>
						<li>
							<code>/gif {@render dim('query')}</code>
							to search GIF suggestions (example:
							<code>/gif happy cat</code>
							)
						</li>
						<li>
							Use <code>Arrow Up/Down</code>
							, then
							<code>Enter</code>
							or
							<code>Tab</code>
							to insert the selected result
						</li>
						<li>
							When mood is unknown, click a suggestion chip or press <code>Tab</code>
							to apply the first suggestion
						</li>
					</ul>
				</div>

				<div>
					<p class="mb-1 font-semibold">Toolbar Shortcuts (6)</p>
					<ul class="space-y-1 list-disc pl-5">
						<li>
							<span class="font-semibold">Header:</span>
							adds
							<code>#</code>
							at the cursor
						</li>
						<li>
							<span class="font-semibold">Bold:</span>
							wraps selection with
							<code>**...**</code>
						</li>
						<li>
							<span class="font-semibold">Italic:</span>
							wraps selection with
							<code>_..._</code>
						</li>
						<li>
							<span class="font-semibold">Code:</span>
							wraps selection with
							<code>`...`</code>
						</li>
						<li>
							<span class="font-semibold">Kaomoji:</span>
							opens the mood-based kaomoji picker drawer
						</li>
						<li>
							<span class="font-semibold">GIF:</span>
							opens the GIF search drawer
						</li>
					</ul>
				</div>
			</div>
		</div>
	</div>
{/if}
