<script>
</script>

<div class="flex flex-col gap-2 xl:gap-4 relative full">
  <div class="stat-grid grid-cols-2 xl:grid-cols-4 gap-2 xl:gap-4">
    <div>
      <h4>Total views</h4>
      <p>0</p>
    </div>
    <div>
      <h4>Total views</h4>
      <p>0</p>
    </div>
    <div>
      <h4>Total views</h4>
      <p>0</p>
    </div>
    <div>
      <h4>Total views</h4>
      <p>0</p>
    </div>
  </div>
  <div class="w-full p-4 bg-white/80 rounded-xl">
    <h3>Recent Activity</h3>
  </div>
</div>

<style lang="postcss">
  @reference "../../app.css";

  .stat-grid {
    @apply grid *:col-span-1 *:rounded-xl *:bg-white/80 *:p-4;
  }

  h4 {
    @apply text-lg font-semibold;
  }

  h3 {
    @apply text-xl font-semibold;
  }
</style>
