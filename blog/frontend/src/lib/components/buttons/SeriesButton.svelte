<script>
  let { fill, ...rest } = $props();
</script>

<button {...rest}>
  <svg
    viewBox="0 0 24 24"
    xmlns="http://www.w3.org/2000/svg"
    x="0px"
    y="0px"
    {fill}
  >
    <g>
      <path
        fill-rule="evenodd"
        clip-rule="evenodd"
        d="M3 4C3 3.44772 3.44772 3 4 3H12H20C20.5523 3 21 3.44772 21 4V12V20C21 20.5523 20.5523 21 20 21H12H4C3.44772 21 3 20.5523 3 20V12V4ZM5 5V11H11V5H5ZM13 5V11H19V5H13ZM19 13H13V19H19V13ZM11 19V13H5V19H11Z"
      />
    </g>
  </svg>
</button>

<style lang="postcss">
  @reference "../../../app.css";

  button {
    @apply not-disabled:hover:cursor-pointer;
  }
</style>
