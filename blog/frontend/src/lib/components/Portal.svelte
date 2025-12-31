<script>
  let { target, children, ...rest } = $props();
  const Portal = (element) => {
    if (!target) return;

    target.appendChild(element);
    return () => element.remove();
  };
</script>

<div {...rest} {@attach Portal}>
  {@render children?.()}
</div>
