<script>
  import { Canvas } from "@threlte/core";
  import * as THREE from "three";
  import GLBLoader from "./GLBLoader.svelte";

  let { name, width = "100%", height = "500px" } = $props();

  let scene = $state();
  let animation = $state(false);
</script>

<div
  class="mx-auto bg-accent-blue rounded-lg overflow-hidden"
  style:max-width={width}
  style:height
>
  <Canvas
    gl={{
      toneMapping: THREE.ACESFilmicToneMapping,
      outputColorSpace: THREE.SRGBColorSpace,
      toneMappingExposure: 1.2,
    }}
  >
    <GLBLoader {name} bind:this={scene} />
  </Canvas>
</div>
