<script>
  import { T } from "@threlte/core";
  import {
    Environment,
    GLTF,
    OrbitControls,
    useDraco,
    useGltf,
    useGltfAnimations,
  } from "@threlte/extras";
  import { AgXToneMapping, LinearToneMapping, SRGBColorSpace } from "three";

  let {
    name,
    cameraConfig,
    toggleWireframe = $bindable(),
    toggleOrbit = $bindable(),
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

<GLTF {name} url={`/models/${name}`} useDraco={true} bind:gltf={$gltf} />
