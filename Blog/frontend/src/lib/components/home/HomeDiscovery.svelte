<script>
    import PostCard from "./PostCard.svelte";

    let tab = $state(1);
    let defaultTags = $state(
        ["programming", "art", "music"].map((tag) => ({
            value: tag,
            selected: false,
        })),
    );
</script>

<div class="space-y-4">
    <div class="flex justify-between">
        <ul id="home-tab" class="text-xl font-medium h-8">
            <li class:left={true} class:selected={tab === 1}>
                <button onclick={() => (tab = 1)}>Discover</button>
            </li>
            <li class:right={true} class:selected={tab === 2}>
                <button onclick={() => (tab = 2)}>Fresh</button>
            </li>
        </ul>
        <div class="text-base flex gap-2 items-center">
            <span>Popular tags: </span>
            <div id="tag-container" class="space-x-2">
                {#each defaultTags as { value, selected }, index}
                    <button
                        class:selected
                        onclick={() =>
                            (defaultTags[index].selected = !selected)}
                        >{value}</button
                    >
                {/each}
            </div>
        </div>
    </div>
    <ul class="grid grid-cols-2 gap-4">
        <li>
            <PostCard
                title="Whispers Beneath the Hollow Sky"
                excerpt="A thin veil of mist drifted across the abandoned field, carrying with it the faint scent of rain and forgotten stories. Mara paused, sensing the quiet hum beneath the earth—a vibration too delicate to hear yet impossible to ignore. Something ancient was waking, and with each heartbeat she felt the world tilt a little closer to the truth she wasn’t ready to face."
                author={{
                    name: "Tèo em",
                    slug: "teo-em",
                }}
                link="/"
                src="missing.png"
                tags={[
                    "game",
                    "programming",
                    "programming",
                    "programming",
                    "programming",
                    "programming",
                ]}
                series={{
                    name: "Introduction",
                    order: 1,
                    slug: "intro",
                }}
            />
        </li>
        <li>
            <PostCard
                title="Whispers Beneath the Hollow Sky Whispers Beneath the Hollow Sky"
                excerpt="A thin veil of mist drifted across the abandoned field, carrying with it the faint scent of rain and forgotten stories. Mara paused, sensing the quiet hum beneath the earth—a vibration too delicate to hear yet impossible to ignore. Something ancient was waking, and with each heartbeat she felt the world tilt a little closer to the truth she wasn’t ready to face."
                author="teo-em"
                link="/"
                src="missing.png"
            />
        </li>
        <li>
            <PostCard
                title="Title"
                excerpt="This is a demo text"
                author="teo-em"
                link="/"
                src="missing.png"
            />
        </li>
        <li>
            <PostCard
                title="Title"
                excerpt="This is a demo text"
                author="teo-em"
                link="/"
                src="missing.png"
            />
        </li>
        <li>
            <PostCard
                title="Title"
                excerpt="This is a demo text"
                author="teo-em"
                link="/"
                src="missing.png"
            />
        </li>
    </ul>
</div>

<style lang="postcss">
    @reference "../../../app.css";

    #home-tab {
        @apply flex gap-4;
        li {
            @apply relative;
            &::after {
                @apply absolute content-[""] bottom-0 h-1 w-0 bg-dark transition-all duration-200;
            }
            &.left::after {
                right: -0.5rem;
            }
            &.right::after {
                left: -0.5rem;
            }
        }
        li.selected::after {
            @apply w-[calc(100%+1rem)];
        }
    }
    #tag-container {
        & > button {
            @apply border-2 border-dark bg-white rounded-full px-2;
            &.selected {
                @apply bg-dark text-white;
            }
        }
    }
</style>
