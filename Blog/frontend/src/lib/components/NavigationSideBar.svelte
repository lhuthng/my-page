<script>
    import { isMod } from "$lib/client/user";
    import AboutButton from "./buttons/AboutButton.svelte";
    import BlogButton from "./buttons/BlogButton.svelte";
    import DashboardButton from "./buttons/DashboardButton.svelte";
    import FacebookButton from "./buttons/FacebookButton.svelte";
    import GithubButton from "./buttons/GithubButton.svelte";
    import HomeButton from "./buttons/HomeButton.svelte";
    import LinkedinButton from "./buttons/LinkedinButton.svelte";
    import ProjectButton from "./buttons/ProjectButton.svelte";

    let { route } = $props();

    const routes = [
        [HomeButton, "Home", "/", ""],
        [BlogButton, "Posts", "/posts", "posts"],
        [ProjectButton, "Projects", "/projects", "projects"],
        [AboutButton, "About", "/about", "about"],
        [DashboardButton, "Dashboard", "/dashboard", "dashboard", true],
    ];
</script>

<div>
    <div class="sticky top-32 w-46 space-y-8">
        <ul class="space-y-2 bg-white/90 p-2 rounded-xl" id="side-bar">
            {#each routes as [Icon, text, path, routeName, secret], index}
                {#if !secret || $isMod}
                    <li class={routeName === route ? "selected" : undefined}>
                        <a href={path}
                            ><Icon
                                class="w-8 transition-all duration-100"
                            />{text}</a
                        >
                    </li>
                {/if}
            {/each}
        </ul>
        <div class="space-y-2 bg-white/90 p-2 rounded-xl">
            <div>
                <span class="block text-center">Connect with me on:</span>
                <div class="flex justify-evenly [&>a]:hover:fill-black/90">
                    <FacebookButton as="a" class="w-10 fill-dark" href="/" />
                    <GithubButton as="a" class="w-10 fill-dark" href="/" />
                    <LinkedinButton as="a" class="w-10 fill-dark" href="/" />
                </div>
            </div>
        </div>
    </div>
</div>

<style lang="postcss">
    @reference "../../app.css";

    #side-bar {
        & > li {
            @apply w-full rounded-lg bg-background/40 hover:bg-background/60 transition-colors text-dark duration-50;

            & > a {
                @apply flex items-center gap-1 px-2 py-1 w-full [&>button]:fill-dark [&>button]:stroke-dark;
            }
        }
        & > li.selected {
            @apply text-white bg-dark hover:bg-dark/90 [&>a>button]:fill-white [&>a>button]:stroke-white;
        }
    }
</style>
