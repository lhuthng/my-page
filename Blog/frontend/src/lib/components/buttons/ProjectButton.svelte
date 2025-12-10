<script>
    let { fill, ...rest } = $props();
</script>

<button {...rest}>
    <svg
        xmlns="http://www.w3.org/2000/svg"
        x="0px"
        y="0px"
        viewBox="0 0 24 24"
        {fill}
    >
        <g>
            <path
                d="M6,6H2V2h4V6z M14,2h-4v4h4V2z M22,2h-4v4h4V2z M6,10H2v4h4V10z M14,10h-4v4h4V10z M22,10h-4v4h4V10z M6,18H2v4h4V18z M14,18h-4v4h4V18z M22,18h-4v4h4V18z"
            />
        </g>
    </svg>
</button>

<style lang="postcss">
    @reference "../../../app.css";

    button {
        @apply not-disabled:hover:cursor-pointer;
    }
</style>
