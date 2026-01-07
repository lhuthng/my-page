<script>
  import { page } from "$app/state";
  import App from "$lib/components/App.svelte";

  let imageUrl = $derived(
    page.url.origin + "/html/l-game/l-game-thumbnail.png",
  );
  let description =
    "Trap your opponent so they cannot make a legal move. Play the classic L-Game strategy board game online.";
</script>

<svelte:head>
  <title>L-Game | Huu Thang's Projects</title>
  <meta name="description" content={description} />
  <link rel="canonical" href={page.url.href} />

  <meta property="og:type" content="website" />
  <meta property="og:url" content={page.url.href} />
  <meta property="og:title" content="Play L-Game Online" />
  <meta property="og:description" content={description} />
  <meta property="og:image" content={imageUrl} />

  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:title" content="Play L-Game Online" />
  <meta name="twitter:description" content={description} />
  <meta name="twitter:image" content={imageUrl} />
</svelte:head>

<div class="w-full pb-4">
  <div class="w-full flex flex-col gap-2 bg-white rounded-xl p-4">
    <h1 class="text-center">L-Game</h1>
    <h3 class="text-center">
      made by <a href="/profiles/lhuthng">@Thắng</a>
    </h3>
    <div class="mx-auto">
      <App name="l-game" type="html" width="396px" height="396px" />
    </div>
    <h3 class="font-semibold">L-Game: Rules & Details</h3>
    <p>The Goal: Trap your opponent so they cannot make a legal move.</p>
    <div>
      <h4>Rules of Play</h4>
      <ol class="list-decimal list-inside">
        <li>
          <span
            >Move the L: On your turn, you must move your L-piece to a new
            position (it can be flipped or rotated).</span
          >
        </li>
        <li>
          <span
            >Move a coin (Optional): After moving your L, you may move one of
            the two neutral coin to any empty square on the grid.</span
          >
        </li>
        <li>
          <span
            >Winning: You win if your opponent is blocked and cannot place their
            L-piece anywhere else on the board.</span
          >
        </li>
      </ol>
    </div>
  </div>
</div>

<style lang="postcss">
  @reference "../../../app.css";
  h1 {
    @apply text-3xl;
  }
  h3 {
    @apply text-xl;
  }
</style>
