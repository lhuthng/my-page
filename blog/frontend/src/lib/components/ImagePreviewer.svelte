<script>
  import { preventDefault, stopPropagation } from "$lib/common";
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { gsap } from "gsap";
  import { Flip } from "gsap/Flip";

  let { visible, origin, onClose } = $props();

  let src = $derived(origin?.src);
  let alt = $derived(origin?.alt);

  let styles = $derived.by(() => {
    if (!origin) return {};

    let { top, left, width, height } = origin.getBoundingClientRect();

    top += "px";
    left += "px";
    width += "px";
    height += "px";

    return { top, left, width, height };
  });

  let root = $state();
  let replica = $state();
  let editable = $state(false);

  const zoomCoef = 0.1;
  let position = $state({
    scale: 1,
    left: 0,
    top: 0,
    width: 0,
    height: 0,
    defaultWidth: 0,
    defaultHeight: 0,
  });
  let touchValue = $state([
    {
      lastX: 0,
      lastY: 0,
    },
    {
      lastX: 0,
      lastY: 0,
    },
  ]);
  let lastTouch = $state(new Map());
  let initialDist = $state(undefined);
  let initialScale = $state(undefined);

  function onZoom({ clientX, clientY }, scale) {
    if (!editable || !root) return;

    const { width, height } = root.getBoundingClientRect();
    const oldScale = position.scale;
    position.width = position.defaultWidth * scale;
    position.height = position.defaultHeight * scale;
    position.scale = scale;

    const x = clientX / width,
      y = clientY / height;
    position.top = y + (position.top - y) * (scale / oldScale);
    position.left = x + (position.left - x) * (scale / oldScale);
    gsap.to(replica, {
      top: (position.top * 100).toFixed(2) + "%",
      left: (position.left * 100).toFixed(2) + "%",
      width: position.width,
      height: position.height,
      duration: 0.1,
    });
  }

  let isDragging = false;
  function onDragging({ movementX, movementY }) {
    if (!editable || !isDragging || !root) return;
    const { width, height } = root.getBoundingClientRect();

    position.left += movementX / width;
    position.top += movementY / height;

    gsap.to(replica, {
      left: (position.left * 100).toFixed(2) + "%",
      top: (position.top * 100).toFixed(2) + "%",
      duration: 0.1,
    });
  }

  onMount(() => {
    if (!root || !replica || !origin) return;

    const { width, height } = root.getBoundingClientRect();
    const state = Flip.getState(replica);

    const { naturalWidth, naturalHeight } = origin;
    const ratio = naturalWidth / naturalHeight;

    let newWidth = naturalWidth,
      newHeight = naturalHeight;

    if (naturalWidth > width || naturalHeight > height) {
      if (width / ratio < height) {
        newWidth = width;
        newHeight = width / ratio;
      } else {
        newHeight = height;
        newWidth = height * ratio;
      }
    }
    position.top = 0.5;
    position.left = 0.5;
    position.width = newWidth;
    position.height = newHeight;

    position.defaultWidth = newWidth;
    position.defaultHeight = newHeight;

    gsap.set(replica, {
      top: "50%",
      left: "50%",
      width: newWidth + "px",
      height: newHeight + "px",
      x: "-50%",
      y: "-50%",
    });

    Flip.from(state, {
      duration: 0.2,
      ease: "power3.in",
      onComplete: () => (editable = true),
    });
  });
</script>

<div
  class="fixed inset-0 bg-dark/80 *:pointer-events-auto overflow-visible touch-none"
  in:fade={{ duration: 200 }}
  out:fade={{ duration: 200 }}
  onwheel={(e) => {
    e.preventDefault();
    const scale = Math.max(0.1, position.scale - (e.deltaY / 100) * zoomCoef);
    onZoom(e, scale);
  }}
  bind:this={root}
  ontouchstart={(e) => {
    for (const t of e.changedTouches) {
      lastTouch.set(t.identifier, {
        x: t.clientX,
        y: t.clientY,
      });
    }
    isDragging = true;

    if (lastTouch.size >= 2) {
      const [a, b, ..._] = [...lastTouch.values()];
      initialDist = Math.hypot(b.x - a.x, b.y - a.y);
      initialScale = position.scale;
    }
  }}
  ontouchmove={(e) => {
    let len = e.changedTouches.length;
    let movementX = 0,
      movementY = 0;
    for (const t of e.changedTouches) {
      const prev = lastTouch.get(t.identifier);
      movementX += t.clientX - prev.x;
      movementY += t.clientY - prev.y;

      prev.x = t.clientX;
      prev.y = t.clientY;
    }
    if (len === 0) return;

    movementX /= len;
    movementY /= len;

    onDragging({ movementX, movementY });
    if (lastTouch.size >= 2) {
      const [a, b, ..._] = [...lastTouch.values()];
      const dist = Math.hypot(b.x - a.x, b.y - a.y);
      onZoom(
        {
          clientX: (a.x + b.x) / 2,
          clientY: (a.y + b.y) / 2,
        },
        initialScale * (dist / (initialDist ?? dist)),
      );
    }
  }}
  ontouchend={(e) => {
    for (const t of e.changedTouches) {
      lastTouch.delete(t.identifier);
    }
    if (lastTouch.size < 2) {
      initialDist = undefined;
    }
  }}
>
  <button
    class="absolute inset-0 cursor-not-allowed! touch-none"
    onclick={onClose}
    aria-label="close-preview"
  >
  </button>

  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <img
    style:top={styles?.top}
    style:left={styles?.left}
    style:width={styles?.width}
    style:height={styles?.height}
    class="absolute rounded-lg cursor-grab pointer-events-auto max-w-none!"
    {src}
    {alt}
    ondragstart={(e) => e.preventDefault()}
    onmousedown={() => (isDragging = true)}
    onmousemove={onDragging}
    onmouseup={(e) => {
      isDragging = false;
    }}
    onmouseleave={(e) => {
      isDragging = false;
    }}
    bind:this={replica}
  />
</div>
