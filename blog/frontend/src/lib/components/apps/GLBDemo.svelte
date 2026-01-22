<script>
  import { Canvas } from "@threlte/core";
  import * as THREE from "three";
  import GLBLoader from "./GLBLoader.svelte";
  import { derived } from "svelte/store";

  let { name, width = "100%", height = "500px", config } = $props();

  let toggleWireframe = $state();
  let toggleOrbit = $state();

  let wireframe = $state(false);
  let animation = $state(false);
  let info = $state(false);
  let orbit = $state(false);

  let cameraConfig = $derived.by(() => {
    let camera = {
      fov: "10",
      position: "0,0,1",
      target: "0,0,0",
      autoRotate: "true",
    };

    if (config) {
      camera = {
        ...camera,
        ...Object.fromEntries(config?.split("-")?.map((s) => s.split(":"))),
      };
    }

    camera.position = camera.position.split(",").map(Number);
    camera.target = camera.target.split(",").map(Number);
    camera.autoRotate = camera.autoRotate === "true";

    return camera;
  });

  $effect(() => {
    orbit = cameraConfig.autoRotate;
  });

  $effect(() => {
    toggleOrbit?.(orbit);
  });

  $effect(() => {
    toggleWireframe?.(wireframe);
  });
</script>

<div
  class="relative mx-auto rounded-lg overflow-hidden"
  class:bg-accent-blue={!wireframe}
  class:bg-black={wireframe}
  style:max-width={width}
  style:height
>
  <Canvas renderMode="on-demand" toneMapping={THREE.NoToneMapping}>
    <GLBLoader {name} {cameraConfig} bind:toggleWireframe bind:toggleOrbit />
  </Canvas>
  <div
    class="absolute top-0 right-0 w-full pointer-events-none has-focus:*:opacity-100! *:transition-opacity *:duration-200"
  >
    <button
      class="absolute right-2 top-2 pointer-events-auto rounded-full w-6 h-6 bg-white opacity-40 mx-auto hover:opacity-70 select-none font-bold"
    >
      i</button
    >
    <div
      class="opacity-0 pointer-events-none mt-10 bg-white rounded-lg mx-2 px-2 py-1"
      class:opacity-100={info}
    >
      <ul class="min-w-60 list-disc select-none">
        <li><strong>Left click/One finger + drag:</strong> Rotate</li>
        <li>
          <strong>Right click/Two finger + drag:</strong> Pan
        </li>
        <li>
          <strong>Middle click + drag, Mouse wheel,</strong> or
          <strong>pinch:</strong> Zoom in/out
        </li>
      </ul>
    </div>
  </div>
  <div class="absolute left-0 bottom-4 w-full flex gap-2 justify-center">
    <div
      class="w-fit duo-btn duration-0!"
      class:duo-blue={!wireframe}
      class:duo-black={wireframe}
    >
      <button class="duration-0!" onclick={() => (wireframe = !wireframe)}
        >Wireframe {wireframe ? "on" : "off"}</button
      >
    </div>
    <div
      class="w-fit duo-btn duration-0!"
      class:duo-blue={!wireframe}
      class:duo-black={wireframe}
    >
      <button class="duration-0!" onclick={() => (orbit = !orbit)}
        >Auto Rotate {orbit ? "on" : "off"}</button
      >
    </div>
  </div>
</div>
