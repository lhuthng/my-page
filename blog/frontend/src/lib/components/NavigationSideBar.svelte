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

<div class="relative not-sm:hidden *:w-12 *:lg:w-46">
    <div></div>
    <div
        class="fixed not-lg:top-16 top-32 space-y-4 lg:space-y-8 transition-transform duration-200"
    >
        <ul class="space-y-2 bg-white/90 p-2 rounded-xl" id="side-bar">
            {#each routes as [Icon, text, path, routeName, secret], index}
                {#if !secret || $isMod}
                    <li class={routeName === route ? "selected" : undefined}>
                        <a href={path}>
                            <Icon class="w-8 transition-all duration-100" />
                            <span class="not-lg:hidden">{text}</span>
                        </a>
                    </li>
                {/if}
            {/each}
        </ul>
        <div class="flex flex-col gap-4 bg-white/90 p-1 lg:p-2 rounded-xl">
            <div class="not-lg:w-10 flex flex-col">
                <span class="block not-lg:hidden text-center"
                    >Connect with me on:</span
                >
                <div
                    class="flex w-full not-lg:flex-col lg:justify-evenly items-center [&>a]:hover:fill-black/90"
                >
                    <FacebookButton
                        as="a"
                        class="w-10 fill-dark"
                        href="https://www.facebook.com/lhuthng/"
                    />
                    <GithubButton
                        as="a"
                        class="w-10 fill-dark"
                        href="https://github.com/lhuthng"
                    />
                    <LinkedinButton
                        as="a"
                        class="w-10 fill-dark"
                        href="https://www.linkedin.com/in/huuthangle/"
                    />
                </div>
            </div>
            <div class="not-lg:hidden px-2 flex flex-col font-medium">
                <span class="font-normal">more: </span>
                <ul class="list-disc list-inside">
                    <li>
                        <a href="https://portfolio.huuthang.site">Portfolio</a>
                    </li>
                    <li><a href="/">About</a></li>
                </ul>
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
                @apply flex items-center gap-1 px-1 lg:px-2 py-1 w-full [&>button]:fill-dark [&>button]:stroke-dark;
            }
        }
        & > li.selected {
            @apply text-white bg-dark hover:bg-dark/90 [&>a>button]:fill-white [&>a>button]:stroke-white;
        }
    }

    a {
        @apply no-underline!;
    }
</style>
