<script>
  import { T } from "@threlte/core";
  import {
    Environment,
    GLTF,
    OrbitControls,
    useGltfAnimations,
  } from "@threlte/extras";
  import { AgXToneMapping, LinearToneMapping, SRGBColorSpace } from "three";

  let {
    name,
    url,
    cameraConfig,
    toggleWireframe = $bindable(),
    toggleOrbit = $bindable(),
    isReady = $bindable(),
  } = $props();

  let { position, fov, target, autoRotate } = $state(cameraConfig);

  let gltfAsset = $state();
  let orbitControl = $state(autoRotate);

  const { gltf, actions, mixer } = useGltfAnimations();

  $effect(() => {
    if (Object.keys($actions).length > 0) {
      Object.values($actions).forEach((clip) => {
        clip.reset().play();
      });
    }
  });

  let gltfUrl = $state();
  $effect(async () => {
    isReady = false;
    if (url) {
      gltfUrl = url;
      isReady = true;
    } else {
      const tryUrl = `/api/media/i/${name}`;
      const res = await fetch(tryUrl);

      if (res.ok) {
        gltfUrl = tryUrl;
        isReady = true;
      }
    }
  });

  toggleWireframe = (value) => {
    if (!$gltf?.scene) return;

    $gltf.scene.traverse((obj) => {
      if (obj.isMesh) {
        obj.material.wireframe = value;
      }
    });
  };

  toggleOrbit = (value) => {
    orbitControl = value;
  };
</script>

<T.PerspectiveCamera makeDefault {position} {fov}>
  <OrbitControls
    autoRotate={orbitControl}
    autoRotateSpeed={1}
    enableDamping
    {target}
  />
</T.PerspectiveCamera>

{#if isReady && gltfUrl}
  <GLTF {name} url={gltfUrl} bind:gltf={$gltf} />
{/if}
