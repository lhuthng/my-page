<script>
  import { goto } from "$app/navigation";
  import { login, logout, register, user } from "$lib/client/user";
  import { onMount } from "svelte";

  import { fly } from "svelte/transition";

  let { data } = $props();

  let isLogged = $derived($user !== undefined);

  let isLogging = $state(!data.register);
  let username = $state("");
  let password = $state("");
  let repassword = $state("");
  let email = $state("");
  let pending = $state(false);
  let status = $state(true);
  let message = $state("");
  let countdown = $state(0);

  $effect(() => {
    isLogging = !data.register;
  });

  let interval, timer;

  async function handleLogin(e) {
    e.preventDefault();
    message = "";
    pending = true;
    const res = await login(username, password);
    pending = false;
    if (!res.status) {
      status = false;
      message = res.message.toLowerCase();
    } else {
      status = true;
      message = "";
      countdown = 3;
      interval = setInterval(() => {
        countdown -= 1;
      }, 1000);
      timer = setTimeout(() => {
        goto("/");
      }, 3500);
    }
  }

  async function handleRegister(e) {
    e.preventDefault();
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

  onMount(() => {
    return () => {
      if (interval) clearInterval(interval);
      if (timer) clearTimeout(timer);
    };
  });
</script>

<div class="flex w-full items-center min-h-[calc(100dvh-8rem)] py-4">
  <div class="mx-auto p-8 rounded-3xl bg-white">
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
        class="space-y-2 *:rounded-xl *:border-2 *:border-dark/40 *:has-focus:border-dark *:bg-primary/20 *:has-disabled:opacity-40 text-dark"
      >
        <div class="px-2">
          <input
            class="py-1.5 w-full"
            placeholder="Username"
            autocomplete="username"
            bind:value={username}
            disabled={isLogged && isLogging}
          />
        </div>
        <div class="flex gap-2 px-2">
          <input
            class="grow py-1.5"
            placeholder="Password"
            type="password"
            bind:value={password}
            disabled={isLogged && isLogging}
          />
          {#if isLogging}
            <button
              type="button"
              class=" text-primary/80 hover:text-dark cursor-pointer"
              disabled={isLogged}
            >
              forgot?
            </button>
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
      <div class="w-full duo-btn duo-primary">
        <button
          class="w-full"
          type="submit"
          disabled={pending || (isLogging && isLogged)}
        >
          {#if isLogging}
            Log In
          {:else}
            Sign Up
          {/if}
        </button>
      </div>
      {#if isLogging && isLogged}
        <div class="flex flex-col items-center w-full">
          <span class="text-accent-green">
            You're are logged as {$user.username}
          </span>
          <span class="text-accent-red">
            Re-direct in {countdown}
          </span>
        </div>
      {/if}
      <div class="separator">
        <span>or</span>
      </div>
      <div class="w-full duo-btn duo-primary">
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
        You don't need to log in to read posts! Create a profile for a cool
        avatar when you comment. <span class="text-nowrap">𐔌՞ ܸ.ˬ.ܸ՞𐦯</span>
      </p>
      {#if !isLogging}
        <p in:fly class="text-lg text-dark/80 text-justify">
          Passwords are hashed using a one-way function and never stored in
          plain text. When you log in, the password you enter is hashed and
          compared to the stored hash - the original password is never stored or
          recoverable <span class="text-nowrap">ヾ(•̀ ヮ &lt;)و</span>.
        </p>
      {/if}
    </form>
  </div>
</div>

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
