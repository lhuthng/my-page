<script>
</script>

<footer class="w-full bg-dark py-2 text-white divide-white">
  <div class="flex not-sm:flex-col w-cap justify-between">
    <div class="lg:w-1/2 pt-4 space-y-2 sm:space-y-4 not-sm:text-center">
      <h2 class="text-2xl">
        <a href="/"
          ><img class="inline h-10" src="/logo.svg" alt="logo-icon" /></a
        > Huu Thang's Blog
      </h2>
      <p>Exploring the intersection of programming and creativity (probably)</p>
    </div>
    <div class="flex grow justify-evenly gap-12 px-0 lg:px-4 py-2 lg:py-4">
      <div class="space-y-2 text-left sm:text-right">
        <h3 class="text-xl">EXPLORE</h3>
        <ul class="space-y-1 list-inside list-['-'] pt-1">
          <li><a href="/posts">Posts</a></li>
          <li><a href="/projects">Projects</a></li>
          <li><a href="/about">About</a></li>
        </ul>
      </div>
      <div class="space-y-2 text-right">
        <h3 class="text-xl">CONNECT</h3>
        <ul class="space-y-1 list-inside list-['-'] pt-1">
          <li><a href="https://github.com/lhuthng">Github</a></li>
          <li>
            <a href="https://www.linkedin.com/in/huuthangle/">Linkedin</a>
          </li>
          <li>
            <a href="https://www.facebook.com/lhuthng/">Facebook</a>
          </li>
          <li>
            <a href="https://www.youtube.com/@memofie">Youtube</a>
          </li>
        </ul>
      </div>
    </div>
  </div>
  <hr class="w-cap border-t-2 my-4" />
  <div class="w-cap not-sm:text-center">
    <span class="mx-auto">
      &copy; 2026 Huu Thang &middot; Hand-craft with Svelte & Rust <span
        class="text-nowrap"
        >[Running on a VPS] &middot; All rights reserved</span
      >.
    </span>
  </div>
</footer>

<style lang="postcss">
  @reference "../../app.css";

  ul > li {
    @apply relative ml-auto w-fit bg-white text-dark pr-1 rounded-r-lg;
  }
  ul > li::before {
    @apply absolute content-[''] bg-white right-full h-full w-3;
    clip-path: polygon(100% 0%, 105% 0%, 105% 100%, 100% 100%, 0% 50%);
  }
</style>
