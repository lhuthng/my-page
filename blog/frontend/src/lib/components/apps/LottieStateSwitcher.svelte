<script>
  import { DotLottieSvelte } from "@lottiefiles/dotlottie-svelte";
  import { onDestroy } from "svelte";

  let { name, states: state_entries, width, height } = $props();

  let states = $state(Object.fromEntries(state_entries));
  let dotLottie = $state();
  let isPlaying = $state(false);

  let selection = $state(state_entries[0][0]);

  function goto(state) {
    if (!dotLottie) return;
    selection = state;
    dotLottie.stop();
    const start = Math.round(dotLottie.currentFrame);
    const end = states[state];
    const direction = start < end ? "forward" : "reverse";

    dotLottie.setMode(direction);

    if (direction === "forward") {
      dotLottie.setSegment(start, end);
    } else {
      dotLottie.setSegment(end, start);
    }
    isPlaying = true;
    dotLottie.play();
  }

  function onComplete() {
    isPlaying = false;
  }

  function setupListeners(dotLottieInstance) {
    dotLottieInstance.addEventListener("complete", onComplete);
  }

  function removeListeners(dotLottieInstance) {
    dotLottieInstance.removeEventListener("complete", onComplete);
  }

  onDestroy(() => {
    if (dotLottie) {
      removeListeners(dotLottie);
    }
  });
</script>

<div class="flex flex-col mx-auto max-w-80 p-4 rounded-xl bg-accent-blue/60">
  <div class="h-80">
    <DotLottieSvelte
      loop={false}
      autoplay={false}
      src={`/lotties/${name}.lottie`}
      dotLottieRefCallback={(ref) => {
        dotLottie = ref;
        setupListeners(dotLottie);
      }}
    />
  </div>

  <div class="flex gap-4 *:w-20 justify-center not-lg:items-center">
    {#each state_entries as [state, _]}
      <div
        class="duo-btn"
        class:duo-blue={selection !== state}
        class:duo-green={selection === state}
      >
        <button disabled={isPlaying} onclick={() => goto(state)}>{state}</button
        >
      </div>
    {/each}
  </div>
</div>
