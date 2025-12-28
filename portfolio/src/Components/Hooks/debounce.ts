import { useEffect, useState } from "react";

export default function useDebounce<T>(fn: T, delay: number) {
    const [ value, setValue ] = useState<T>(fn);

    useEffect(() => {
        const handler = setTimeout(() => setValue(fn), delay);
        return () => clearTimeout(handler);
    }, [fn, delay]);

    return value;
}