export function createBoundedEase(
    maxValue: number,
    specialDist: number,
    specialValue: number,
) {
    const c = (specialDist * (maxValue - specialValue)) / specialValue;
    return (dist: number) => {
        return maxValue * dist / (dist + c)
    };
}
