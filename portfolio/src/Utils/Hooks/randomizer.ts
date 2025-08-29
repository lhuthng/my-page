export default function useRandomizer<T>(pool: T[]) {
    let last: T | undefined = undefined;
    return (avoidLast: boolean = false) => {
        let item: T | undefined = undefined;
        do {
            item = pool[Math.floor(Math.random() * pool.length)];
        } while (avoidLast && pool.length != 0 && item === last);

        last = item;
        return item;
    };
}