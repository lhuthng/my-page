<script>
  import App from "../App.svelte";
  import { findHeaders, pluginExtend } from "$lib/custom-rules";

  let { content, headers = $bindable() } = $props();

  function applyPlugins(e) {
    pluginExtend(e);
    headers = findHeaders(e);
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
    & a {
      @apply text-accent-blue mr-2;
    }
    & a::after {
      content: "^";
      @apply absolute inline-block align-middle text-sm -rotate-30;
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
      content: counter(h1) ". ";
    }
    & h2 {
      @apply mt-4 text-xl font-bold;
      counter-reset: h3;
    }
    & h2::before {
      @apply font-bold;
      counter-increment: h2;
      content: counter(h1) "." counter(h2) ". ";
    }
    & h3 {
      @apply mt-1 text-lg font-semibold;
      counter-reset: h4;
    }
    & h3::before {
      @apply font-bold;
      counter-increment: h3;
      content: counter(h2) ". " counter(h3) ". ";
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
    /*& pre {
      @apply bg-dark rounded-sm px-2 py-1;
    }
    & pre > code {
      @apply text-white;
    }*/
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
    & .reveal {
      @apply flex flex-col mt-4 p-2 text-white bg-dark max-h-11 overflow-y-hidden rounded-xl border-2 overflow-x-hidden;
    }
    & .reveal > .reveal-content {
      @apply opacity-0 -translate-y-4 pointer-events-none transition-all duration-200;
    }
    & .reveal > .reveal-tooltip {
      @apply w-full;
    }
    & .reveal.toggled {
      @apply max-h-full;
    }

    & .reveal.toggled > .reveal-content {
      @apply opacity-100 translate-y-0 pointer-events-auto;
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
      @apply py-2 w-full;
    }
    & .audio-container > audio {
      @apply mx-auto rounded-full border-secondary border-2;
    }
    & .video-container {
      @apply w-full py-4;
    }
    & .video-container > video {
      @apply mx-auto overflow-hidden rounded-lg;
    }
    & .audio-sync-container {
      @apply mx-auto w-fit border-2 border-secondary bg-secondary px-5 pb-4 rounded-[2.8rem];
    }
  }
</style>
