<script>
    import { login, logout, register, user } from "$lib/client/user";

    let isLogged = $derived($user !== undefined);

    let isLogging = $state(true);
    let username = $state("");
    let password = $state("");
    let repassword = $state("");
    let email = $state("");
    let pending = $state(false);
    let status = $state(true);
    let message = $state("");

    async function handleLogin() {
        message = "";
        pending = true;
        const res = await login(username, password);
        pending = false;
        if (!res.status) {
            status = false;
            message = res.message.toLowerCase();
        } else {
            status = true;
            message = "user logged in sucessfully!";
        }
    }

    async function handleRegister() {
        if (password !== repassword) {
            status = false;
            message = "repassword does not match.";
            return;
        }
        message = "";
        pending = true;
        const res = await register(username, password, email);
        pending = false;
        if (!res.status) {
            status = false;
            message = res.message.toLowerCase();
        } else {
            status = true;
            message = "user signed up sucessfully!";
        }
    }

    let handleSubmit = $derived(isLogging ? handleLogin : handleRegister);

    $effect(() => {
        isLogging;
        message = "";
    });
</script>

<div class="flex w-full items-center min-h-[calc(100dvh-8rem)]">
    <div class="mx-auto p-8 rounded-3xl bg-white/80">
        <form
            class="flex flex-col gap-4 w-80 *:items-center text-xl"
            onsubmit={handleSubmit}
            novalidate
        >
            <h3 class="mx-auto py-2">
                {#if isLogging}
                    Log In
                {:else}
                    Sign Up
                {/if}
            </h3>
            <div
                class="space-y-2 *:rounded-xl *:border-2 *:border-dark/40 *:has-focus:border-dark *:bg-primary/20 text-dark"
            >
                <div class="px-2">
                    <input
                        class="py-1.5"
                        placeholder="Username"
                        autocomplete="username"
                        bind:value={username}
                    />
                </div>
                <div class="flex gap-2 px-2">
                    <input
                        class="flex-1 py-1.5"
                        placeholder="Password"
                        type="password"
                        bind:value={password}
                    />
                    {#if isLogging}
                        <button
                            type="button"
                            class=" text-primary/80 hover:text-dark cursor-pointer"
                            >forgot?</button
                        >
                    {/if}
                </div>
                {#if !isLogging}
                    <div class="flex px-2">
                        <input
                            class="grow py-1.5"
                            placeholder="Re-password"
                            type="password"
                            bind:value={repassword}
                        />
                    </div>
                    <div class="flex px-2">
                        <input
                            class="grow py-1.5"
                            placeholder="Email"
                            type="email"
                            bind:value={email}
                        />
                    </div>
                {/if}
            </div>
            {#if message}
                <div class="w-full">
                    <span
                        class="text-right"
                        class:text-accent-red={!status}
                        class:text-accent-green={status}>*{message}</span
                    >
                </div>
            {/if}
            <div class="w-full duo-btn duo-dark">
                <button class="w-full" type="submit" disabled={pending}>
                    {#if isLogging}
                        Log In
                    {:else}
                        Sign Up
                    {/if}
                </button>
            </div>
            <div class="separator">
                <span>or</span>
            </div>
            <div class="w-full duo-btn duo-dark">
                <button
                    type="button"
                    class="w-full"
                    onclick={() => (isLogging = !isLogging)}
                >
                    {#if isLogging}
                        Sign Up
                    {:else}
                        Log In
                    {/if}
                </button>
            </div>
            <p class="text-lg text-dark/80 text-justify">
                You don't need to log in to read posts! Create a profile for a
                cool avatar when you comment. 𐔌՞ ܸ.ˬ.ܸ՞𐦯
            </p>
        </form>
    </div>
</div>

<!-- <div class="flex flex-col w-cap h-screen min-h-200">
    <div class="flex bg-green-200 w-full h-160 my-auto">
        <div class="grow"></div>
        <div class="w-120 bg-green-300 h-full p-4">
            {#if !isLogged}
                <form
                    class="flex flex-col bg-green-400 gap-2 [&>input]:bg-green-500"
                    onsubmit={handleSubmit}
                >
                    <input
                        id="f_username"
                        type="text"
                        placeholder="username"
                        bind:value={username}
                    />
                    <input
                        id="f_password"
                        type="text"
                        placeholder="password"
                        bind:value={password}
                    />
                    {#if !isLogging}
                        <input
                            id="f_repassword"
                            type="text"
                            placeholder="repassword"
                            bind:value={repassword}
                        />
                        <input
                            id="f_email"
                            type="text"
                            placeholder="email"
                            bind:value={email}
                        />
                        <div>
                            <label
                                >Already have an account? <button
                                    type="button"
                                    onclick={() => (isLogging = true)}
                                    >Login!</button
                                ></label
                            >
                        </div>
                    {:else}
                        <div>
                            <label
                                >Don't have an account? <button
                                    type="button"
                                    onclick={() => (isLogging = false)}
                                    >Signup!</button
                                ></label
                            >
                        </div>
                    {/if}
                    <input
                        type="submit"
                        value={isLogging ? "Login" : "Signup"}
                    />
                </form>
            {:else}
                <div>
                    You're already logged. <button onclick={() => logout()}
                        >Log out?</button
                    >
                </div>
            {/if}
        </div>
    </div>
</div> -->

<style lang="postcss">
    @reference "../../app.css";

    h3 {
        @apply text-2xl font-bold;
    }

    .separator {
        @apply flex w-full gap-4 items-center relative text-dark/20;

        &::before {
            @apply content-[''] grow h-0.5 bg-dark/20;
        }
        &::after {
            @apply content-[''] grow h-0.5 bg-dark/20;
        }
    }
</style>
