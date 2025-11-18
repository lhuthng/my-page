<script>
    let { details } = $props();

    let shortNameValue = $state(details.short_name);
    let descriptionValue = $state(details.description);

    $effect(() => {
        shortNameValue = details.short_name;
        descriptionValue = details.description;
    });
</script>

<form
    class="flex flex-col"
    method="patch"
>
    <label for="short-name">
        Short name: <input type="text" bind:value={shortNameValue} name="short-name"/>
    </label>
    <label for="description">
        Description: <input type="text" bind:value={descriptionValue} name="description"/>
    </label>
    <label for="file-type">
        File type: {details.file_type}
    </label>
    <label for="aliases">
        Aliases:
    </label>
</form>
