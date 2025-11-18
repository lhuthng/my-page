<script>
    let { size, file, isSelected, onclick, ondblclick } = $props();

    let width = $state(size);
    let height = $state(size);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
    class={`flex flex-col h-fit text-center rounded-sm gap-2 mx-auto p-2 cursor-pointer ${isSelected ? "bg-gray-100" : ""}`}
    {onclick}
    {ondblclick}
    style:max-width={`${size + 20}px`}
>
    <div class="mx-auto rounded-sm drop-shadow-2xl" style:width={`${size}px`}>
        <img
            class="object-contain w-full rounded-sm overflow-hidden"
            style:height={`${size}px`}
            src={file.url}
            alt={file.name}
        />
    </div>
    <span
        class={`wrap-break-word line-clamp-2 ${isSelected ? "select-text" : "select-none"}`}
        >{file.name}</span
    >
</div>
