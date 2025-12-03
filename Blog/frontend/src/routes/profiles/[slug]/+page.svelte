<script>
    import Club from "$lib/components/svgs/Club.svelte";
    import Diamond from "$lib/components/svgs/Diamond.svelte";
    import Heart from "$lib/components/svgs/Heart.svelte";
    import Spade from "$lib/components/svgs/Spade.svelte";

    const { data } = $props();

    let response = $derived(data.response);
    let username = $derived(response.username);
    let displayName = $derived(response.display_name);
    let role = $derived(response.role);
    let bio = $derived(response.bio);
</script>

<section class="bg-white/90 rounded-xl p-4" key={username}>
    <div class="flex gap-4">
        <div class="space-y-4 min-w-60">
            <img
                class="w-60 h-60 rounded-bl-xl rounded-tr-xl"
                src="/missing.png"
                alt="avatar"
            />
            <div class="grid grid-cols-2">
                <div class="duo-btn duo-blue mx-2">
                    <button disabled>Message</button>
                </div>
                <div class="duo-btn duo-green mx-2">
                    <button disabled>Follow</button>
                </div>
            </div>
        </div>
        <div>
            <div class="flex items-end">
                <h2 class="inline font-bold text-3xl">{displayName}</h2>
                <span
                    class="*:w-10 hover:-translate-y-1 transition-all duration-200 tooltip-container"
                    data-tooltip={role}
                >
                    {#if role === "admin"}
                        <Heart class="fill-accent-red" />
                    {:else if role === "moderator"}
                        <Diamond class="fill-accent-red" />
                    {:else if role === "user"}
                        <Club class="fill-dark" />
                    {:else}
                        <Spade class="fill-dark" />
                    {/if}
                </span>
            </div>
            <span class="italic text-primary/80 *:select-none"
                ><span>./</span>{username}<span>/</span></span
            >
            <p class="py-4 text-lg">{bio}</p>
        </div>
    </div>
</section>
