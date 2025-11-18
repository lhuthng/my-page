export const useDebounce = (updater, delay) => {
    let debouncedTimeout;

    return {
      update: (value) => {
          clearTimeout(debouncedTimeout);
          debouncedTimeout = setTimeout(() => updater?.(value), delay);
      },
      destroy: () => clearTimeout(debouncedTimeout)
    }
}
