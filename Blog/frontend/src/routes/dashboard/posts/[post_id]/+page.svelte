<script>
    const { data } = $props();

    $effect(() => {
        console.log(data);
    });
</script>

<div></div>
