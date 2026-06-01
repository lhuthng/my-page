export function resizeTextarea(node) {
  if (!node) return;

  node.style.height = "auto";
  node.style.height = `${node.scrollHeight}px`;
}

export function autoHResize(node) {
  const setHeight = () => resizeTextarea(node);

  setHeight();
  node.addEventListener("input", setHeight);

  return {
    update() {
      setHeight();
    },
    destroy() {
      node.removeEventListener("input", setHeight);
    },
  };
}
