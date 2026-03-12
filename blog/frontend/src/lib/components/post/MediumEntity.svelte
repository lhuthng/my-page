<script>
  let { shortName, url, type, warning, changeName } = $props();
  let draft = $state(shortName);
  let skip = false;
  let playable = $state();
  let timeline = $state();
  let isPlaying = $state(false);

  $effect(() => {
    isPlaying = false;
    switch (playable?.tagName) {
      case "AUDIO": {
        const play = () => {
          isPlaying = true;
        };
        const pause = () => {
          isPlaying = false;
        };
        const timeupdate = () => {
          const current = playable.currentTime; // seconds played
          const total = playable.duration; // total length (seconds)
          const percent = (current / total) * 100;
          timeline.style.width = percent + "%";
        };
        const complete = () => {
          isPlaying = false;
        };
        playable.addEventListener("play", play);
        playable.addEventListener("pause", pause);
        playable.addEventListener("timeupdate", timeupdate);
        playable.addEventListener("complete", complete);
        return () => {
          playable.removeEventListener("play", play);
          playable.removeEventListener("pause", pause);
          playable.removeEventListener("timeupdate", timeupdate);
          playable.removeEventListener("complete", complete);
        };
      }
    }
  });
</script>

<li
  class="flex items-center gap-2 p-2 rounded-lg bg-primary/20 hover:brightness-105"
  class:bg-yellow-200={warning}
>
  <div class="flex w-8 lg:w-10 h-8 lg:h-10">
    {#if type.startsWith("image")}
      <img
        class="m-auto max-w-8 lg:max-w-10 max-h-8 lg:max-h-10 object-contain rounded-sm overflow-hidden"
        src={url}
        alt={shortName}
      />
    {:else if type.startsWith("audio")}
      <div
        class="relative audio-container m-auto w-8 lg:w-10 h-8 lg:h-10 object-contain rounded-full border-3 outline-primary/60 bg-primary/20 overflow-hidden"
      >
        <audio bind:this={playable} src={url} alt={shortName}></audio>
        <div
          bind:this={timeline}
          class="absolute z-9 left-0 h-full bg-primary/60"
        ></div>
        <button
          class="relative full z-10 p-3 hover:*:scale-120"
          onclick={() => {
            if (playable?.isPlaying) playable?.pause();
            else playable?.play();
          }}
          ondblclick={() => {
            playable?.pause();
            if (playable) playable.currentTime = 0;
          }}
          title="audio-button"
        >
          {#if !isPlaying}
            <svg
              class="full translate-x-0.5 fill-white"
              x="0px"
              y="0px"
              viewBox="0 0 460.114 460.114"
            >
              <g>
                <g>
                  <path
                    d="M393.538,203.629L102.557,5.543c-9.793-6.666-22.468-7.372-32.94-1.832c-10.472,5.538-17.022,16.413-17.022,28.26v396.173 c0,11.846,6.55,22.721,17.022,28.26c10.471,5.539,23.147,4.834,32.94-1.832l290.981-198.087 c8.746-5.954,13.98-15.848,13.98-26.428C407.519,219.477,402.285,209.582,393.538,203.629z"
                  />
                </g>
              </g>
            </svg>
          {:else}
            <svg class="full fill-white" x="0px" y="0px" viewBox="0 0 32 32">
              <path
                d="M14,6v20c0,1.1-0.9,2-2,2H8c-1.1,0-2-0.9-2-2V6c0-1.1,0.9-2,2-2h4C13.1,4,14,4.9,14,6z M24,4h-4c-1.1,0-2,0.9-2,2v20c0,1.1,0.9,2,2,2h4c1.1,0,2-0.9,2-2V6C26,4.9,25.1,4,24,4z"
              />
            </svg>
          {/if}
        </button>
      </div>
    {:else if type.startsWith("model")}
      <svg
        class="m-auto w-8 h-8 lg:w-10 lg:h-10 text-primary"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linejoin="round"
        stroke-linecap="round"
        aria-label="3d-model"
        role="img"
      >
        <path d="M12 2 3.5 6.5 12 11 20.5 6.5 12 2Z" />
        <path d="M3.5 6.5V17.5L12 22V11" />
        <path d="M20.5 6.5V17.5L12 22" />
        <path d="M12 11 20.5 6.5" />
      </svg>
    {/if}
  </div>
  {#if changeName}
    <input
      class="focus:outline-none focus:border-b"
      type="text"
      bind:value={draft}
      onblur={() => {
        if (!skip) {
          draft = shortName;
        }
        skip = false;
      }}
      onkeydown={(e) => {
        if (e.key === "Enter") {
          if (changeName(shortName, draft)) {
            skip = true;
          }
          e.target.blur();
        }
      }}
    />
  {:else}
    <span>{shortName}</span>
  {/if}
</li>
