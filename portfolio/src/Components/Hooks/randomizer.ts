export default function useRandomizer<T>(pool: T[]) {
    return () => pool[Math.floor(Math.random() * pool.length)];
}