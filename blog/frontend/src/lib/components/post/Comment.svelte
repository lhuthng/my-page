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
