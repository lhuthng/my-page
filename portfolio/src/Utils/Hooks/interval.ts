export default function useInterval(): [
    (exec: () => boolean, delay: number) => void, 
    () => void,
    (callback: () => void) => void
] {

    let interval: ReturnType<typeof setInterval> | null = null;
    let _callback: (() => void) | undefined = undefined;

    const onStopped = () => {
        if (interval !== null) {
            clearInterval(interval);
            interval = null;
        }
    }

    const onStarted = (exec: () => boolean, delay: number) => {
        interval = setInterval(() => {
            if (exec()) {
                onStopped();
                _callback?.();
            }
        }, delay);
    }

    const onEnded = (callback: () => void) => {
        _callback = callback;
    }

    return [
        onStarted,
        onStopped,
        onEnded
    ]
}