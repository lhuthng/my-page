<script>
    const { content } = $props();
</script>

<div class="rendered-markdown">{@html content}</div>

<style lang="postcss">
    @reference "../../../app.css";

    :global(.rendered-markdown) {
        & {
            @apply space-y-1;
        }
        & h1 {
            @apply text-3xl;
        }
        & img {
            @apply rounded-lg mx-auto;
        }
    }
</style>
