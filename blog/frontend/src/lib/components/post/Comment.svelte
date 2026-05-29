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
      @apply w-fit min-h-6;
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
      @apply text-accent-blue hover:text-accent-blue-light-1 underline;
    }
    & a.mention-link {
      @apply relative inline-flex items-center gap-1 rounded-md bg-primary-20 px-1.5 py-0.5 no-underline text-primary hover:bg-primary/30;
    }
    & .mention-chip {
      @apply font-semibold;
    }
    & .mention-preview {
      @apply pointer-events-none absolute left-0 bottom-full z-50 mb-2 hidden w-max min-w-max items-center gap-2 whitespace-nowrap rounded-xl bg-primary p-2 shadow-xl;
    }
    & a.mention-link:hover .mention-preview,
    & a.mention-link:focus-visible .mention-preview {
      @apply inline-flex;
    }
    & .mention-preview-avatar {
      @apply h-10 w-10 rounded-lg object-cover m-0! max-h-max! border-2;
    }
    & .mention-preview-name {
      @apply block flex-none whitespace-nowrap text-sm font-semibold text-white;
    }
    & img {
      @apply mt-2 max-h-40 rounded-lg mr-auto;
    }
    & ul {
      @apply list-disc pl-4;
    }
    & li > ul,
    & li > ol {
      @apply ml-0.5;
    }
    & ol {
      @apply list-decimal list-inside;
    }
    & table {
      @apply my-6 mx-auto border-2 border-dark;
    }
    & thead {
      @apply bg-dark/90 border-2 border-dark text-white;
    }
    & tr {
      @apply text-left;
    }
    & td {
      @apply p-1 not-first:border-l-2;
    }
    & th {
      @apply p-1 not-first:border-l-2 border-dark;
    }
    & p > code,
    & li > code {
      @apply bg-gray-200 p-1 rounded-md hover:brightness-95;
    }
    & blockquote {
      @apply relative ml-4 pl-4;
    }
    & blockquote::after {
      content: "";
      @apply absolute top-0 left-0 h-full w-0.5 bg-dark;
    }
    & code {
      @apply whitespace-pre-wrap wrap-break-word break-after-all inline not-sm:text-sm;
    }
  }
</style>
