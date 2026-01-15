<script>
  import { DotLottieSvelte } from "@lottiefiles/dotlottie-svelte";

  import { onDestroy, onMount } from "svelte";

  let canvas = $state();
  let dotLottie = $state();
  let isPlaying = $state(false);
  const states = {
    seed: 0,
    sprout: 29,
    mature: 59,
  };
  let selection = $state("seed");

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

<div class="bg-white rounded-lg p-4">
  <div class="flex flex-col mx-auto max-w-80 p-4 rounded-xl bg-accent-blue/60">
    <div class="h-80">
      <DotLottieSvelte
        loop={false}
        autoplay={false}
        src="/lotties/my-flower.lottie"
        dotLottieRefCallback={(ref) => {
          dotLottie = ref;
          setupListeners(dotLottie);
        }}
      />
    </div>

    <div class="flex gap-4 *:w-20 justify-center not-lg:items-center">
      <div
        class="duo-btn"
        class:duo-blue={selection !== "seed"}
        class:duo-green={selection === "seed"}
      >
        <button disabled={isPlaying} onclick={() => goto("seed")}>Seed</button>
      </div>

      <div
        class="duo-btn"
        class:duo-blue={selection !== "sprout"}
        class:duo-green={selection === "sprout"}
      >
        <button disabled={isPlaying} onclick={() => goto("sprout")}
          >Sprout</button
        >
      </div>
      <div
        class="duo-btn"
        class:duo-blue={selection !== "mature"}
        class:duo-green={selection === "mature"}
      >
        <button disabled={isPlaying} onclick={() => goto("mature")}
          >Mature</button
        >
      </div>
    </div>
  </div>
</div>
