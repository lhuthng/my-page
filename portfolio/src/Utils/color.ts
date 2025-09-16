const colors = [ 
    "white", 
    "black", 
    "dark-charcoal", 
    "yellow", 
    "orange", 
    "blue", 
    "cyan", 
    "navi", 
    "green", 
    "aquamarine", 
    "lime", 
    "pink", 
    "purple", 
    "silver",
    "dark-gray",
    "light-gray", 
    "orange-red", 
    "salmon", 
    "violet" 
] as const;
export type Color = typeof colors[number];

export function toRGB(color: Color, alpha: number = 1) {
    return `rgba(var(--${color}-chalk), ${alpha})`;
}