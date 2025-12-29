<script>
    import { pbody } from "$lib/client/misc";
    import Portal from "./Portal.svelte";

    let { children } = $props();
</script>

<Portal
    class={`full bg-dark/50 ${!children ? "hidden" : "block"}`}
    target={$pbody}
>
    {@render children?.()}
</Portal>
