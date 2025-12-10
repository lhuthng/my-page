<script>
    let { size, file, isSelected, ok, onclick, ondblclick } = $props();
</script>

<button
    class={`flex flex-col h-fit text-center rounded-sm gap-2 mx-auto p-2 cursor-pointer ${isSelected ? "bg-gray-100" : ""}`}
    {onclick}
    {ondblclick}
    style:max-width={`${size + 20}px`}
>
    <div
        class="flex items-center justify-center rounded-sm drop-shadow-2xl"
        style:width={`${size}px`}
        style:height={`${size}px`}
    >
        <div class="flex" style:height={`${size}px`} style:width={`${size}px`}>
            <div class="m-auto relative">
                <img
                    class="m-auto object-contain rounded-sm overflow-hidden"
                    style:max-height={`${size}px`}
                    style:max-width={`${size}px`}
                    src={file.url}
                    alt={file.name}
                />
                {#if ok === true}
                    <svg
                        class="absolute right-1 bottom-1 w-4 h-4"
                        width="0px"
                        height="0px"
                        viewBox="0 0 48 48"
                        xmlns="http://www.w3.org/2000/svg"
                    >
                        <circle fill="#4CAF50" cx="24" cy="24" r="21" />

                        <g fill="none" stroke="#FFFFFF" stroke-width="4">
                            <line x1="15.4" y1="22.6" x2="21" y2="28.2" />
                            <line x1="21" y1="28.2" x2="34.6" y2="14.6" />
                        </g>
                    </svg>
                {:else if ok === false}
                    <svg
                        class="absolute right-1 bottom-1 w-4 h-4"
                        width="0px"
                        height="0px"
                        viewBox="0 0 48 48"
                        xmlns="http://www.w3.org/2000/svg"
                    >
                        <circle fill="#F44336" cx="24" cy="24" r="21" />
                        <g fill="none" stroke="#FFFFFF" stroke-width="4">
                            <line x1="15" y1="15" x2="33" y2="33" />
                            <line x1="33" y1="15" x2="15" y2="33" />
                        </g>
                    </svg>
                {/if}
            </div>
        </div>
    </div>
    <span
        class={`wrap-break-word line-clamp-2 ${isSelected ? "select-text" : "select-none"}`}
        >{file.name}</span
    >
</button>
