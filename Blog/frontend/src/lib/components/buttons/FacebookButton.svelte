<script>
    let { as = "button", ...rest } = $props();
</script>

<svelte:element this={as} {...rest}>
    <svg viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg" x="0px" y="0px"
        ><defs
            ><mask fill="black" id="fb-mask">
                <path fill="white" d="M0 0h16v16H0z" />
                <path
                    d="M8.789 4.723c.387 0 1.041.006 1.041.006V3.415s-.455-.011-1.052 0-2.064.057-2.064 1.786v1.115H5.577v1.365h1.137v4.265H8.34V7.681h1.331l.159-1.365H8.34V5.201s.063-.478.449-.478"
                />
            </mask>
        </defs>
        <path
            d="M3.209.52h8.941c1.479 0 2.69 1.21 2.69 2.69v8.941c0 1.479-1.21 2.69-2.69 2.69H3.209a2.7 2.7 0 0 1-2.69-2.69V3.21A2.697 2.697 0 0 1 3.209.521Z"
            mask="url(#fb-mask)"
        />
    </svg>
</svelte:element>
