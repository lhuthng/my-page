<script>
	let { content } = $props();
</script>

<div class="rendered-comment-markdown">
	{@html content}
</div>

<style lang="postcss">
	@reference "../../../app.css";

	:global(.rendered-comment-markdown) {
		& {
			@apply block min-h-6 w-full max-w-full min-w-0;
			overflow-wrap: anywhere;
			word-break: break-word;
		}
		& hr {
			@apply mt-4 border-t-2 border-dashed;
		}
		& h1 {
			@apply text-2xl font-bold;
		}
		& h2,
		& h3,
		& h4,
		& h5,
		& h6 {
			@apply text-xl font-semibold;
		}
		& a {
			@apply text-accent-blue underline hover:text-accent-blue-light-1;
			overflow-wrap: anywhere;
			word-break: break-word;
		}
		& a.mention-link {
			@apply relative inline-flex items-center gap-1 rounded-md bg-primary-20 px-1 text-primary no-underline hover:bg-primary/30;
		}
		& .mention-chip {
			@apply font-semibold;
		}
		& .mention-preview {
			@apply pointer-events-none absolute bottom-full left-0 z-50 mb-2 hidden w-max min-w-max items-center gap-2 rounded-xl bg-primary p-2 whitespace-nowrap shadow-xl;
		}
		& a.mention-link:hover .mention-preview,
		& a.mention-link:focus-visible .mention-preview {
			@apply inline-flex;
		}
		& .mention-preview-avatar {
			@apply m-0! h-10 max-h-max! w-10 rounded-lg border-2 object-cover;
		}
		& .mention-preview-name {
			@apply block flex-none text-sm font-semibold whitespace-nowrap text-white;
		}
		& img {
			@apply mt-2 mr-auto max-h-40 max-w-full rounded-lg;
		}
		& ul {
			@apply list-disc pl-4;
		}
		& li > ul,
		& li > ol {
			@apply ml-0.5;
		}
		& ol {
			@apply list-inside list-decimal;
		}
		& table {
			@apply my-6 block max-w-full overflow-x-auto border-2 border-dark;
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
			overflow-wrap: anywhere;
			word-break: break-word;
		}
		& blockquote {
			@apply relative ml-4 pl-4;
		}
		& blockquote::after {
			content: '';
			@apply absolute top-0 left-0 h-full w-0.5 bg-dark;
		}
		& code {
			@apply inline wrap-break-word whitespace-pre-wrap not-sm:text-sm;
			overflow-wrap: anywhere;
			word-break: break-word;
		}
		& p,
		& li,
		& blockquote,
		& pre {
			overflow-wrap: anywhere;
			word-break: break-word;
		}
		& span.kaomoji {
			display: inline-block;
			width: max-content;
			max-width: 100%;
			white-space: normal;
		}
	}
</style>
