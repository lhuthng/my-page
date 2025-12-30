<script>
  let { content } = $props();
</script>

<div class="rendered-markdown">{@html content}</div>

<style lang="postcss">
  @reference "../../../app.css";

  :global(.rendered-markdown) {
    & {
      @apply space-y-1;
    }
    & a {
      @apply text-accent-blue mr-2;
    }
    & a::after {
      content: "↝";
      @apply absolute inline-block align-middle text-sm -rotate-30;
    }
    & h1 {
      @apply mt-6 text-2xl font-bold;
    }
    & h2 {
      @apply mt-4 text-xl font-bold;
    }
    & h3 {
      @apply mt-1 text-lg font-semibold;
    }
    & img {
      @apply rounded-lg mx-auto;
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
    & pre {
      @apply bg-dark rounded-sm px-2 py-1;
    }
    & pre > code {
      @apply text-white;
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
