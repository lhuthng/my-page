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
  import { ACESFilmicToneMapping, SRGBColorSpace } from "three";

  let { name } = $props();

  let gltfAsset = $state();

  const { gltf, actions, mixer } = useGltfAnimations();

  $effect(() => {
    if (Object.keys($actions).length > 0) {
      Object.values($actions).forEach((clip) => {
        clip.reset().play();
      });
    }
  });
</script>

<T.PerspectiveCamera makeDefault position={[0, 20, 55]} fov={30}>
  <OrbitControls
    autoRotate
    autoRotateSpeed={1}
    enableDamping
    target={[0, 8, 0]}
  />
</T.PerspectiveCamera>

<T.AmbientLight intensity={0.28} color="#ffffff" />
<T.DirectionalLight position={[5, 10, 5]} intensity={2.6} color="#ffffff" />

<T.WebGLRenderer
  parameters={{
    antialias: true,
    alpha: false,
  }}
  toneMapping={ACESFilmicToneMapping}
  toneMappingExposure={0.9}
  outputColorSpace={SRGBColorSpace}
/>

<Environment background={false} preset="studio" />

<GLTF {name} url="/models/demo.glb" useDraco={true} bind:gltf={$gltf} />
