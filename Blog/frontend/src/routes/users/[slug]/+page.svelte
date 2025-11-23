<script>
    let { data } = $props();

    const { username, display_name: displayName, role, bio } = data;
</script>

<div class="w-cap py-20">
    <div class="w-full space-y-2">
        <div class="flex">
            <div class="grow divide-y">
                <h1 class="text-5xl">{displayName}</h1>
                <p>{bio}</p>
            </div>
            <div class="w-80 h-80 bg-red-500 rounded-full"></div>
        </div>
        <div class="w-full h-20 bg-blue-500">
            <div>
                <div class="flex justify-between">
                    <h2>Posts {role}</h2>
                    {#if role in ["moderator", "admin"]}
                        <div>New</div>
                    {/if}
                </div>
            </div>
        </div>
    </div>
</div>
