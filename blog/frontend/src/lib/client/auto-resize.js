export function autoHResize(node) {
    function setHeight() {
        node.style.height = "auto";
        node.style.height = node.scrollHeight + "px";
    }

    setHeight();
    node.addEventListener("input", setHeight);

    return {
        destroy() {
            node.removeEventListener("input", setHeight);
        },
    };
}
