<script>
    let { fill, ...rest } = $props();
</script>

<button {...rest}>
    <svg
        xmlns="http://www.w3.org/2000/svg"
        x="0px"
        y="0px"
        viewBox="0 0 32 32"
        {fill}
    >
        <path
            d="M25,27H7c-2.2,0-4-1.8-4-4V9c0-2.2,1.8-4,4-4h18c2.2,0,4,1.8,4,4v14C29,25.2,27.2,27,25,27z"
        />
        <rect x="8" y="11" width="6" height="6" />
        <line x1="18" y1="17" x2="24" y2="17" />
        <line x1="8" y1="21" x2="24" y2="21" />
    </svg>
</button>

<style lang="postcss">
    @reference "../../../app.css";
    button {
        @apply not-disabled:hover:cursor-pointer;
    }
    path,
    rect,
    line {
        fill: none;
        stroke-width: 2.5;
        stroke-linecap: round;
        stroke-linejoin: round;
    }
</style>
