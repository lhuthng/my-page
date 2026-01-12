<script>
  import {
    auth,
    changeAvatarUrl,
    changeDisplayname as changeDisplayName,
    user,
  } from "$lib/client/user.js";
  import PostCard from "$lib/components/home/PostCard.svelte";
  import Club from "$lib/components/svgs/Club.svelte";
  import Diamond from "$lib/components/svgs/Diamond.svelte";
  import Heart from "$lib/components/svgs/Heart.svelte";
  import Spade from "$lib/components/svgs/Spade.svelte";
  import { onMount } from "svelte";
  import { flip } from "svelte/animate";
  import { fade, fly } from "svelte/transition";
  import { autoHResize } from "$lib/client/auto-resize.js";
  import PBody from "$lib/components/PBody.svelte";
  import { preventDefault } from "$lib/common.js";

  const { data } = $props();

  const allowedTypes = ["image/jpeg", "image/png", "image/gif", "image/webp"];

  let response = $derived(data.response);
  let username = $derived(response.username);
  let role = $derived(response.role);
  let displayName = $state("");
  let bio = $state("");
  let postContainer = $state();
  let avatarUrl = $state("/missing.png");

  $effect(() => {
    avatarUrl = data.response.avatar_url ?? "/missing.png";
    displayName = data.response.display_name;
    bio = data.response.bio;
  });

  let editor = $state({
    isEditing: false,
    isFetching: false,
    isChangingAvatar: false,
    isUploading: false,
    isUploaded: false,
    newAvatar: undefined,
    avatarFile: undefined,
    avatarError: "",
    displayName: "",
    bio: "",
  });

  $effect(() => {
    if (editor.isEditing && $user?.username !== username) {
      editor.isEditing = false;
    }
  });

  const hasPosts = $derived(role === "moderator" || role === "admin");

  let posts = $state({
    status: "initial",
    fetchingMore: false,
    fetchedAll: false,
    message: "",
    data: [],
  });

  const limit = 3;

  const maxFileSize = 5 * 1024 * 1024;

  const appendMedia = (files) => {};

  const fetchMore = async () => {
    if (posts.fetchedAll || posts.data.fetchingMore) return;

    const before = posts.data.length;
    posts.data.fetchingMore = true;

    const res = await fetch(
      `/api/users/${username}/posts?limit=${limit}&offset=${before}`,
    );

    if (!res.ok) {
      posts.status = "failed";
      posts.message = await res.text();
    } else {
      const data = (await res.json()).posts;

      if (data.length < limit) {
        posts.fetchedAll = true;
      }

      posts.data = [...posts.data, ...data];
      posts.fetchingMore = false;
    }
  };

  $effect(async () => {
    if (!hasPosts) {
      return;
    }

    posts.status = "fetching";

    const init = {
      method: "GET",
      headers:
        $user?.username === username ? { Authorization: auth() } : undefined,
    };

    const res = await fetch(
      `/api/users/${username}/posts?limit=${limit}&offset=0`,
      init,
    );

    if (!res.ok) {
      posts.status = "failed";
      posts.message = await res.text();
    } else {
      posts.status = "fetched";
      const data = (await res.json()).posts;
      if (data.length < limit) {
        posts.fetchedAll = true;
      }
      posts.data = data;
    }
  });
</script>

{#if editor.isChangingAvatar}
  <PBody>
    <div
      class="sticky top-0 flex justify-center items-center w-full h-screen pointer-events-auto"
    >
      <div
        class="absolute cursor-not-allowed inset-0 z-10"
        onclick={() => {
          editor.isChangingAvatar = false;
          editor.avatarFile = undefined;
          editor.newAvatar = undefined;
          editor.isUploading = false;
          editor.isUploaded = false;
          editor.avatarError = "";
        }}
        role="none"
      ></div>
      <div
        class="w-fit h-fit space-y-4 bg-white rounded-3xl p-4 text-xl z-11"
        role="none"
      >
        <div
          class="flex justify-center items-center w-60 h-60 bg-background/60 outline-4 outline-dark outline-dashed rounded-xl overflow-hidden"
          ondrop={(e) => {
            e.preventDefault();
            const file = e.dataTransfer.files[0];
            if (!file) {
              editor.avatarError = "File not found!";
              return;
            }

            if (!allowedTypes.includes(file.type)) {
              editor.avatarError = "Only JPEG, PNG, GIF, or WEBP are allowed.";
              return;
            }

            if (file.size > maxFileSize) {
              editor.avatarError = `File size exceeds 5MB (${file.size} bytes)`;
              return;
            }

            editor.avatarFile = file;
            editor.newAvatar = URL.createObjectURL(file);

            editor.isUploading = false;
            editor.isUploaded = false;
          }}
          ondragover={preventDefault}
          role="none"
        >
          {#if editor.newAvatar}
            <img
              class="full object-cover"
              src={editor.newAvatar}
              alt="temporal-avatar"
            />
          {:else}
            <span class="w-40 text-center select-none text-dark"
              >Upload your image here</span
            >
          {/if}
        </div>
        {#if editor.avatarError}
          <span class="inline-block w-60 text-accent-red">
            *{editor.avatarError}
          </span>
        {/if}
        <div class="duo-btn duo-green">
          <button
            disabled={!editor.newAvatar ||
              editor.isUploaded ||
              editor.isUploading}
            onclick={async () => {
              const formData = new FormData();

              formData.append(
                "file",
                editor.avatarFile,
                editor.avatarFile.name,
              );

              editor.isUploaded = false;
              editor.isUploading = true;
              const res = await fetch("/api/users/me/avatar", {
                method: "PATCH",
                headers: {
                  Authorization: auth(),
                },
                body: formData,
              });

              if (res.ok) {
                avatarUrl = editor.newAvatar;
                changeAvatarUrl(avatarUrl);
                editor.isUploaded = true;
                editor.isUploading = false;
                editor.avatarError = "";
              } else {
                editor.isUploading = false;
                editor.avatarError = await res.text();
              }
            }}>Apply</button
          >
        </div>
        {#if editor.isUploaded}
          <span class="inline-block w-60 text-accent-green">
            *New avatar uploaded succesfully!
          </span>
        {/if}
      </div>
    </div>
  </PBody>
{/if}

<section
  class="flex flex-col gap-4 *:bg-white *:rounded-xl *:p-4 pb-8"
  key={username}
>
  <div class="flex not-lg:flex-col gap-4">
    <div class="space-y-4 lg:min-w-60">
      <div class="relative overflow-hidden">
        {#if $user?.username === username}
          <div
            class="absolute left-0 bottom-0 w-full h-full opacity-0 hover:opacity-100 bg-background/30 transition-opacity duration-200"
          >
            <div
              class="absolute bottom-0 left-0 flex items-center justify-center w-full py-2 bg-dark/30"
            >
              <div class="duo-btn duo-green">
                <button onclick={() => (editor.isChangingAvatar = true)}
                  >Change Avatar</button
                >
              </div>
            </div>
          </div>
        {/if}
        <img
          class="mx-auto w-full max-w-60 max-h-60 rounded-bl-xl rounded-tr-xl object-cover"
          src={avatarUrl}
          alt="avatar"
        />
      </div>
      <div class="flex justify-evenly gap-2 lg:*:grow not-lg:*:max-w-40">
        <div class="duo-btn duo-blue">
          <button disabled>Message</button>
        </div>
        <div class="duo-btn duo-green">
          <button disabled>Follow</button>
        </div>
      </div>
    </div>
    <div class="w-full">
      <div class="flex items-end">
        {#if editor.isEditing}
          <input
            class="font-bold text-3xl bg-accent-green/20 field-sizing-content"
            type="text"
            name="username"
            autocomplete="off"
            bind:value={editor.displayName}
          />
        {:else}
          <h2 class="inline font-bold text-3xl">{displayName}</h2>
        {/if}
        <span
          class="*:w-10 hover:*:translate-x-1 *:transition-all *:duration-200 tooltip-container"
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
        {#if $user?.username === username}
          <span class="grow"></span>
          <div
            class="duo-btn col-span-full"
            class:duo-green={!editor.isEditing}
            class:duo-red={editor.isEditing}
          >
            <button
              onclick={() => {
                const isEditing = !editor.isEditing;
                if (isEditing) {
                  editor.bio = bio;
                  editor.displayName = displayName;
                }
                editor.isEditing = isEditing;
              }}
              disabled={editor.isFetching}
              >{editor.isEditing ? "Cancel" : "Edit"}</button
            >
          </div>
        {/if}
      </div>
      <span class="italic text-primary/80 *:select-none"
        ><span>./</span>{username}<span>/</span></span
      >
      {#if editor.isEditing}
        <div class="py-4 bg-accent-green/20 rounded-lg">
          <textarea
            name="bio"
            class="text-lg w-full overflow-hidden resize-none outline-none"
            bind:value={editor.bio}
            use:autoHResize
          ></textarea>
          <div class="flex justify-end">
            <div class="duo-btn duo-green w-fit">
              <button
                onclick={async () => {
                  editor.isFetching = true;
                  const res = await fetch("/api/users/me/details", {
                    method: "PATCH",
                    headers: {
                      Authorization: auth(),
                      "Content-Type": "application/json",
                    },
                    body: JSON.stringify({
                      display_name: editor.displayName,
                      bio: editor.bio,
                    }),
                  });
                  editor.isFetching = false;
                  editor.isEditing = false;
                  if (res.ok) {
                    displayName = editor.displayName;
                    bio = editor.bio;
                    const data = [...posts.data];
                    data.forEach((post) => (post.author_name = displayName));
                    posts.data = [...data];
                    changeDisplayName(displayName);
                  }
                }}
                disabled={editor.isFetching}>Submit</button
              >
            </div>
          </div>
        </div>
      {:else}
        <p class="py-4 text-lg whitespace-pre-wrap">{bio}</p>
      {/if}
    </div>
  </div>
  {#if hasPosts}
    <div class="flex flex-col gap-4">
      <h2 class="inline font-bold text-2xl">Posts</h2>
      {#if posts.status === "fetching"}
        <div class="flex justify-center items-center p-4 text-xl">
          Loading...
        </div>
      {:else if posts.status === "failed"}
        <div class="flex justify-center items-center p-4 text-xl">
          Failed to load the posts, try refreshing the pages!
        </div>
      {:else if posts.status === "fetched"}
        <ul
          bind:this={postContainer}
          class="grid grid-cols-1 sm:grid-cols-[repeat(auto-fill,minmax(25rem,1fr))] gap-4"
        >
          {#each posts.data as { id, title, slug, excerpt, author_name, author_slug, tag_slugs, status, url }, index (slug)}
            <li in:fly={{ y: -20, duration: 500 }} out:fade={{ duration: 150 }}>
              <PostCard
                id={status === "draft" ? id : undefined}
                {status}
                {title}
                {slug}
                {excerpt}
                author={{
                  name: author_name,
                  slug: author_slug,
                }}
                tags={tag_slugs}
                src={url}
              />
            </li>
          {/each}
        </ul>
        <div class="flex col-span-full full items-center justify-center">
          <div class="duo-btn duo-green">
            <button
              disabled={posts.fetchingMore || posts.fetchedAll}
              onclick={fetchMore}
              >{posts.fetchingMore
                ? "loading"
                : posts.fetchedAll
                  ? "no more to load"
                  : "load more"}</button
            >
          </div>
        </div>
      {/if}
    </div>
  {/if}
</section>
