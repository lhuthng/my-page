export function useBoundedEase(maxValue: number, specialDist: number, specialValue: number) {
    const c = (specialDist * (maxValue - specialValue)) / specialValue;
    return (dist: number) => {
        return maxValue * dist / (dist + c)
    };
}