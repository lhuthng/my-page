<script>
  import { user } from "$lib/client/user.js";

  let { data } = $props();

  const { username, display_name: displayName, role, bio } = data;

  let isAuthor = $derived(
    ["admin", "moderator"].includes(role) && username === $user?.username,
  );
</script>

<div class="w-cap py-20">
  <div class="w-full space-y-2">
    <div class="flex">
      <div class="grow divide-y">
        <h1 class="text-5xl">
          {displayName}
          {#if isAuthor}
            <span>(you)</span>
          {/if}
        </h1>
        <p>{bio}</p>
      </div>
      <div class="w-80 h-80 bg-red-500 rounded-full"></div>
    </div>
    <div class="w-full h-20 bg-blue-500">
      <div>
        <div class="flex justify-between">
          <h2>Posts</h2>
          {#if isAuthor}
            <a href="/dashboard/posts/new">new</a>
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>
