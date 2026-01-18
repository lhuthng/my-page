<script>
  import { page } from "$app/state";
  import { isPointInTriangle, preventDefault } from "$lib/common";
  import { fly } from "svelte/transition";

  let portionSelection = $state(-1);
  let locked = $state(false);
  let lockedSelection = $state(-1);
  const sqrt3o2 = 0.8660254037844386;
  let imageUrl = $derived(page.url.origin + "/thinkcats.jpg");

  function handleMouseMove(e) {
    const rect = e.currentTarget.getBoundingClientRect();
    const style = window.getComputedStyle(e.currentTarget);
    const paddingLeft = parseFloat(style.paddingLeft);
    const paddingTop = parseFloat(style.paddingTop);

    const mouseX = e.clientX - rect.left - paddingLeft;
    const mouseY = e.clientY - rect.top - paddingTop;

    const width = rect.width - paddingLeft * 2;
    const height = sqrt3o2 * width;
    const triangle = [width / 2, 0, 0, height, width, height];
    const isInside = isPointInTriangle(mouseX, mouseY, ...triangle);
    if (isInside) {
      const center = [width / 2 - 2, (height * 2) / 3 - 5];
      const delta = [mouseY - center[1], mouseX - center[0]];

      let angle = Math.atan2(...delta);
      angle = angle * (180 / Math.PI) + 30;
      if (angle < 0) angle += 360;

      portionSelection = (angle / 120) >> 0;
    } else portionSelection = -1;
  }

  function lock(portion) {
    if (!locked || lockedSelection !== portion) {
      locked = true;
      lockedSelection = portion;
    } else {
      locked = false;
      lockedSelection = -1;
    }
  }

  $effect(() => {
    $inspect(portionSelection, portionSelection !== -1);
  });
</script>

<svelte:head>
  <title>About | Huu Thang's Blog</title>
  <meta
    name="description"
    content="A personal archive documenting software architecture, creative coding, and hands-on experiments by Thắng."
  />

  <meta property="og:type" content="website" />
  <meta property="og:url" content={page.url.origin} />
  <meta property="og:title" content="About | Huu Thang's Blog" />
  <meta
    property="og:description"
    content="Exploring the intersection of systems architecture, 3D motion, and experimental code."
  />
  <meta property="og:image" content={imageUrl} />

  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:image" content={imageUrl} />
  <meta
    name="twitter:description"
    content="Documenting experiments in tech and art."
  />

  <link rel="canonical" href={page.url.href} />
</svelte:head>

{#snippet personaButton(src, alt, selection)}
  <button
    class:opacity-60={locked && lockedSelection !== selection}
    class:z-10!={portionSelection === selection}
    class:scale-105={portionSelection === selection}
    class:scale-110!={lockedSelection === selection && locked}
    class:cursor-pointer!={portionSelection === selection}
    onclick={() => {
      if (portionSelection === selection) lock(selection);
    }}
  >
    <img {src} {alt} />
  </button>
{/snippet}

<div class="bg-white rounded-lg mb-4">
  <div class="flex flex-col gap-2 p-4">
    <h1 class="font-bold text-2xl">About</h1>
    <h2 class="font-semibold text-center text-xl">Welcome back to the Field</h2>
    <p>
      I’m Thắng, and this site serves as my digital garden. It is a personal
      archive where I document my work experiences, my evolving hobbies, and the
      various experiments I run in my spare time. Everything you see here is a
      reflection of my curiosity—whether it’s a deep dive into a technical
      challenge or a small creative spark.
    </p>
    <p>
      I wanted a central place to capture the process, rather than just the
      finished result. To keep these diverse interests organized, I have
      categorized my work under three distinct personas:
    </p>
  </div>
  <div class="flex flex-col xl:flex-row justify-center bg-black w-full">
    <div>
      <div
        class="relative grid place-items-center area-center mx-auto max-w-140 p-8 *:origin-[50%_66%] *:transition-transform *:duration-100 *:z-9"
        onmousemove={handleMouseMove}
        onmouseleave={() => (portionSelection = -1)}
        role="separator"
      >
        {@render personaButton("thinkcat-0.webp", "artistic-cat", 2)}
        {@render personaButton("thinkcat-1.webp", "gaming-cat", 1)}
        {@render personaButton("thinkcat-2.webp", "systematic-cat", 0)}
      </div>
      <div class="flex justify-center">
        <span class="text-white text-center">Click the cats to explore</span>
      </div>
    </div>
    {#if lockedSelection === 0}
      <div
        in:fly={{ x: 100, duration: 200 }}
        class="flex flex-col mt-2 xl:max-w-120 text-justify gap-2 text-white p-4"
      >
        <a class="not-xl:mx-auto" href="/profiles/memo-fie">
          <h2 class="w-fit font-semibold px-2 py-1 bg-red-500 rounded-full">
            Memory Field
          </h2>
        </a>
        <p>
          The Memory "Field" is a dedicated space for all my pure creative works
          and artistic side projects. It serves as the "juice" of this digital
          garden - the core persona responsible for keeping even the driest
          technical subjects feeling lively and moist.
        </p>

        <p>
          This field captures the intersection of programming and creativity
          through a wide range of mediums, including:
        </p>
        <ul class="list-disc list-inside">
          <li>
            <strong>Visual Arts 🎨:</strong> Spanning from hand-drawn raster arts
            and clean vector-based designs to the nostalgic precision of pixel-art.
          </li>
          <li>
            <strong>3D {"&"} Motion 🖼️:</strong> Exploring depth through 3D modeling
            and bringing static ideas to life with animation.
          </li>
          <li>
            <strong>Audio {"&"} Performance 🎶:</strong> Composing music backgrounds
            and custom sound effects, or sharing the process of performing them.
          </li>
        </ul>
        <p>
          Everything in this space is a reflection of my curiosity and the small
          creative sparks that make the work feel truly alive.
        </p>
      </div>
    {:else if lockedSelection === 1}
      <div
        in:fly={{ x: 100, duration: 200 }}
        class="flex flex-col mt-2 xl:max-w-120 text-justify gap-2 text-white p-4"
      >
        <a class="not-xl:mx-auto" href="/profiles/lhuthng">
          <h2 class="w-fit font-semibold px-2 py-1 bg-red-500 rounded-full">
            Thắng
          </h2>
        </a>
        <p>
          Thắng is my personal account for sharing direct experiences,
          unfiltered thoughts, and hands-on experiments. While the other
          personas focus on the how or the creative juice, this space is about
          the person behind the keyboard.
        </p>
        <p>
          It is the most human corner of this digital garden, where I document:
        </p>
        <ul class="list-disc list-inside">
          <li>
            <strong
              >Direct Experiences <span class="text-nowrap">( ´ ▿ ` )</span
              >:</strong
            > Real-world reflections on my journey, my work experiences, and the lessons
            learned along the way.
          </li>
          <li>
            <strong
              >Unfiltered Thoughts <span class="text-nowrap">{"☆(>ᴗ•)/"}</span
              >:</strong
            > A place for raw ideas and "small creative sparks" before they are polished
            into a final system or project.
          </li>
          <li>
            <strong
              >Hands-on Experiments <span class="text-nowrap">(°▽°)</span
              >:</strong
            > Documentation of the various experiments I run in my spare time, reflecting
            my pure curiosity.
          </li>
        </ul>
        <p>
          This section captures the process of learning and living, ensuring the
          site remains a true personal archive rather than just a static
          portfolio.
        </p>
      </div>
    {:else if lockedSelection === 2}
      <div
        in:fly={{ x: 100, duration: 200 }}
        class="flex flex-col mt-2 xl:max-w-120 text-justify gap-2 text-white p-4"
      >
        <a class="not-xl:mx-auto" href="/profiles/admin">
          <h2 class="w-fit font-semibold px-2 py-1 bg-red-500 rounded-full">
            The Architect
          </h2>
        </a>
        <p>
          The Architect is the persona responsible for system architecture,
          technical implementation, and platform infrastructure. This section
          serves as a technical repository for engineering documentation and
          system management.
        </p>
        <p>The core focus areas include:</p>
        <ul class="list-disc list-inside">
          <li>
            <strong>System Structure:</strong> Documentation of the platform's codebase
            and structural design.
          </li>
          <li>
            <strong>Technical Implementation:</strong> Detailed "how-why" guides and
            deep dives into specific engineering challenges.
          </li>
          <li>
            <strong>Infrastructure Management:</strong> Oversight of the platform's
            deployment.
          </li>
        </ul>
        <p>
          While other personas address personal or creative content, The
          Architect ensures the underlying systems remain stable, documented,
          and functional.
        </p>
      </div>
    {/if}
  </div>
  <div class="flex not-xl:flex-col">
    <div class="space-y-2 p-4">
      <h2 class="font-bold text-xl">Tech Stack (Infrastructure)</h2>
      <p>
        The entire ecosystem is hosted on a VPS, utilizing a decoupled
        architecture to handle different functional requirements:
      </p>
      <ul class="list-disc list-inside">
        <li>
          <strong>Blog (this site):</strong> Built with Svelte 5 / SvelteKit 2
          and Rust, containerized with Docker, and utilizing SQLite for data
          persistence.
          <img
            class="w-60 h-auto mx-auto rounded-lg"
            src="stack.webp"
            alt="web-stack"
          />
        </li>
        <li>
          <strong>Portfolio:</strong> A dedicated static site built with React, Lottie,
          and GSAP for high-performance animations
        </li>
        <li>
          <strong>WebSocket Server:</strong> Managed via Node.js for my apps' real-time
          communication
        </li>
        <li>
          <strong>Mail Server: </strong>Custom-built using Rust
        </li>
      </ul>
      <div class="flex gap-2 w-full justify-center fill-dark *:w-8">
        <svg viewBox="0 0 16 16">
          <g>
            <path
              d="M12.438 2.94656C13.0222 3.84625 13.0826 4.82176 12.784 5.56064C12.2332 5.04017 11.5732 4.66735 10.8806 4.48388C10.8888 4.33095 10.8504 4.17412 10.7606 4.03584C10.535 3.68845 10.0705 3.58972 9.72314 3.81532L8.0472 4.90369C8.04673 4.90399 8.04627 4.90429 8.0458 4.9046L5.52979 6.53851C5.1824 6.76411 5.08367 7.22861 5.30926 7.57599C5.53486 7.92338 5.99936 8.02212 6.34675 7.79652L8.86347 6.16214C8.86323 6.16229 8.86371 6.16198 8.86347 6.16214C9.81822 5.54278 11.3592 5.87563 12.2475 7.24351C13.1361 8.61173 12.813 10.1553 11.8583 10.7753L6.82625 14.0431C5.87156 14.6631 4.33001 14.3304 3.44148 12.9622C2.85722 12.0625 2.79684 11.087 3.09545 10.3481C3.64623 10.8686 4.30624 11.2414 4.99882 11.4249C4.99062 11.5778 5.02903 11.7347 5.11882 11.8729C5.34442 12.2203 5.80892 12.3191 6.15631 12.0935L10.3497 9.37027C10.697 9.14467 10.7958 8.68017 10.5702 8.33279C10.3446 7.9854 9.88009 7.88666 9.5327 8.11226L7.01644 9.74634C7.01652 9.74629 7.01635 9.7464 7.01644 9.74634C6.06172 10.3661 4.52038 10.0334 3.63192 8.66527C2.74339 7.29705 3.06648 5.75348 4.02117 5.13349L9.0532 1.86566C10.0079 1.24567 11.5494 1.57834 12.438 2.94656ZM13.7667 6.88194C14.7218 5.56301 14.6705 3.63028 13.696 2.1296C12.4789 0.255528 10.0607 -0.577139 8.23624 0.607651L3.20422 3.87549C1.52381 4.96675 1.20205 7.21441 2.11271 9.02685C1.15769 10.3458 1.20893 12.2785 2.18348 13.7792C3.40052 15.6533 5.81879 16.4859 7.64321 15.3011L12.6752 12.0333C14.3556 10.942 14.6774 8.69437 13.7667 6.88194Z"
            >
            </path>
          </g>
        </svg>
        <svg viewBox="0 0 16 16"
          ><g>
            <path
              d="M8 .1c.422 0 .765.342.765.765v.916c.296.035.587.091.87.166l.283-.683a.764.764 0 111.413.585l-.276.665c.287.157.56.335.817.533l.633-.633a.765.765 0 011.081 1.081l-.62.62c.24.298.453.618.637.956l.631-.261a.765.765 0 11.585 1.413l-.654.27c.072.277.126.56.16.849h.81a.765.765 0 010 1.529h-.81a6.334 6.334 0 01-.21 1.027l.664.275a.765.765 0 11-.585 1.413l-.672-.278c-.13.24-.278.471-.44.693l.504.504a.765.765 0 01-1.081 1.081l-.465-.465c-.3.253-.627.48-.978.677l.269.65a.765.765 0 11-1.413.586l-.27-.65a6.332 6.332 0 01-.883.181v.57a.765.765 0 01-1.53 0v-.555a6.386 6.386 0 01-1.087-.218l-.278.672a.765.765 0 11-1.413-.585l.285-.688a6.38 6.38 0 01-.84-.581l-.407.406a.765.765 0 01-1.081-1.081l.428-.428a6.348 6.348 0 01-.459-.703l-.748.31a.765.765 0 01-.585-1.413l.748-.31a6.339 6.339 0 01-.205-1.09H.865a.765.765 0 010-1.53h.762c.044-.298.108-.593.192-.88l-.81-.336a.765.765 0 01.585-1.413l.815.337c.181-.33.39-.643.625-.934l-.62-.62a.765.765 0 011.081-1.081l.633.633c.205-.158.42-.303.645-.435l-.316-.763a.765.765 0 011.413-.585L6.175 2a6.34 6.34 0 011.06-.22V.865C7.235.442 7.578.1 8 .1zm-.765 3.354a4.854 4.854 0 00-3.487 2.36l.61.199a.765.765 0 11-.472 1.454l-.653-.212a4.823 4.823 0 001.294 4.225l.458-.63a.764.764 0 011.237.898l-.47.648A4.818 4.818 0 008 12.948c.812 0 1.577-.2 2.249-.552l-.352-.484a.765.765 0 011.237-.899l.34.467c.349-.36.642-.773.867-1.227a4.843 4.843 0 00.34-2.97l-.567.184a.764.764 0 11-.472-1.454l.5-.163a4.827 4.827 0 00-3.377-2.374v.447a.765.765 0 11-1.53 0v-.469z"
            >
            </path>
          </g>
        </svg>
        <svg viewBox="0 0 16 16"
          ><g>
            <path
              d="M5.46619 2.00322C5.46619 1.44916 5.91535 1 6.46941 1H9.47907C10.0331 1 10.4823 1.44915 10.4823 2.00322V5.01288C10.4823 5.09948 10.4713 5.18352 10.4507 5.26368C10.4713 5.34385 10.4823 5.42789 10.4823 5.51449V8.02254H11.7363V6.0161C11.7363 5.60055 12.0732 5.26368 12.4887 5.26368C12.9043 5.26368 13.2411 5.60055 13.2411 6.0161V8.02254H15.2476C15.6631 8.02254 16 8.35941 16 8.77495C16 9.1905 15.6631 9.52737 15.2476 9.52737H13.2113C13.0634 11.3518 12.3727 12.8078 11.2779 13.8375C10.045 14.9972 8.37374 15.5467 6.59481 15.5467C4.97058 15.5467 3.43674 15.0891 2.24224 14.1255C1.03896 13.1549 0.236443 11.7155 0.0125343 9.86728C-0.115917 8.80699 0.763545 8.02254 1.70412 8.02254H1.95492V5.51449C1.95492 4.96043 2.40408 4.51127 2.95814 4.51127H5.46619V2.00322ZM5.46619 6.0161H3.45975V8.02254H5.46619V6.0161ZM1.70412 9.52737C1.63196 9.52737 1.57447 9.55683 1.54064 9.59191C1.51066 9.62299 1.50235 9.65252 1.50644 9.68629C1.68777 11.1831 2.31696 12.2524 3.18705 12.9543C4.06593 13.6632 5.24713 14.0419 6.59481 14.0419C8.071 14.0419 9.34667 13.5882 10.2469 12.7414C11.0115 12.0222 11.5598 10.9686 11.7007 9.52737H1.70412ZM6.97102 8.02254H8.97746V6.0161H6.97102V8.02254ZM6.97102 4.51127H8.97746V2.50483H6.97102V4.51127Z"
            ></path>
          </g></svg
        >
        <svg viewBox="0 0 16 16"
          ><g>
            <path
              fill-rule="nonzero"
              clip-rule="nonzero"
              d="M0 1.75C0 0.783501 0.783502 0 1.75 0H14.25C15.2165 0 16 0.783502 16 1.75V3.75C16 4.16421 15.6642 4.5 15.25 4.5C14.8358 4.5 14.5 4.16421 14.5 3.75V1.75C14.5 1.61193 14.3881 1.5 14.25 1.5H1.75C1.61193 1.5 1.5 1.61193 1.5 1.75V14.25C1.5 14.3881 1.61193 14.5 1.75 14.5H15.25C15.6642 14.5 16 14.8358 16 15.25C16 15.6642 15.6642 16 15.25 16H1.75C0.783501 16 0 15.2165 0 14.25V1.75ZM4.75 6.5C4.75 6.08579 5.08579 5.75 5.5 5.75H9.25C9.66421 5.75 10 6.08579 10 6.5C10 6.91421 9.66421 7.25 9.25 7.25H8.25V12.5C8.25 12.9142 7.91421 13.25 7.5 13.25C7.08579 13.25 6.75 12.9142 6.75 12.5V7.25H5.5C5.08579 7.25 4.75 6.91421 4.75 6.5ZM11.2757 6.58011C11.6944 6.08164 12.3507 5.75 13.25 5.75C14.0849 5.75 14.7148 6.03567 15.1394 6.48481C15.4239 6.78583 15.4105 7.26052 15.1095 7.54505C14.8085 7.82958 14.3338 7.81621 14.0493 7.51519C13.9394 7.39898 13.7204 7.25 13.25 7.25C12.7493 7.25 12.5306 7.41836 12.4243 7.54489C12.2934 7.70065 12.25 7.896 12.25 8C12.25 8.104 12.2934 8.29935 12.4243 8.45511C12.5306 8.58164 12.7493 8.75 13.25 8.75C13.3257 8.75 13.3988 8.76121 13.4676 8.78207C14.1307 8.87646 14.6319 9.17251 14.9743 9.58011C15.3684 10.0493 15.5 10.604 15.5 11C15.5 11.396 15.3684 11.9507 14.9743 12.4199C14.5556 12.9184 13.8993 13.25 13 13.25C12.1651 13.25 11.5352 12.9643 11.1106 12.5152C10.8261 12.2142 10.8395 11.7395 11.1405 11.4549C11.4415 11.1704 11.9162 11.1838 12.2007 11.4848C12.3106 11.601 12.5296 11.75 13 11.75C13.5007 11.75 13.7194 11.5816 13.8257 11.4551C13.9566 11.2993 14 11.104 14 11C14 10.896 13.9566 10.7007 13.8257 10.5449C13.7194 10.4184 13.5007 10.25 13 10.25C12.9243 10.25 12.8512 10.2388 12.7824 10.2179C12.1193 10.1235 11.6181 9.82749 11.2757 9.41989C10.8816 8.95065 10.75 8.396 10.75 8C10.75 7.604 10.8816 7.04935 11.2757 6.58011Z"
            ></path>
          </g></svg
        >
      </div>
      <p>
        Detailed documentation regarding the architectural decisions, deployment
        pipelines, and the evolution of this stack will be covered in the series <a
          class="text-nowrap font-semibold text-accent-blue"
          href="/series/the-artifact-manifest">[ THE ARTIFACT MANIFEST ]</a
        >. You can find the initial blueprint
        <a class="font-semibold text-accent-blue" href="/posts/i-made-a-blog"
          >here</a
        >.
      </p>
    </div>
    <div class="xl:min-w-120 space-y-2 p-4">
      <h2 class="font-bold text-xl">Professional Links</h2>
      <ul>
        <li>
          Portfolio: <a
            class="font-semibold text-accent-blue"
            href="https://portfolio.huuthang.site">portfolio.huuthang.site</a
          >
        </li>
        <li>
          GitHub: <a
            class="font-semibold text-accent-blue"
            href="https://github.com/lhuthng">github.com/lhuthng</a
          >
        </li>
        <li>
          LinkedIn: <a
            class="font-semibold text-accent-blue"
            href="https://www.linkedin.com/in/huuthangle/"
            >linkedin.com/in/huuthangle</a
          >
        </li>
      </ul>
      <h2 class="font-bold text-xl">Tools (if anyone wonders)</h2>
      <ul>
        <li>
          <strong>Editors:</strong> Zed (primary), Visual Studio Code (ssh-remote),
          Visual Studio.
        </li>
        <li>
          <strong>Illustration &amp; Animation:</strong> Aseprite, Clip Studio Paint,
          Adobe Illustrator, Boxy SVG Editor, Lottie Creator.
        </li>
        <li><strong>Database Manager:</strong> Beekeeper Studio</li>
        <li><strong>Most Important:</strong> Terminal</li>
      </ul>
    </div>
  </div>
</div>

<style lang="postcss">
  @reference "../../app.css";

  .area-center > button {
    @apply cursor-default;
    grid-area: 1 / 1 / 1 / 1;
  }
</style>
