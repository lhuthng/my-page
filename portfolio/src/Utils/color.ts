export type Color = 
    "white" |
    "black" |
    "dark-charcoal" |
    "yellow" |
    "yellow-2" |
    "orange" |
    "orange-2" |
    "blue" |
    "cyan" |
    "navi" |
    "green" |
    "aquamarine" |
    "lime" |
    "pink" |
    "purple" |
    "purple-2" |
    "silver" |
    "dark-gray" |
    "light-gray" |
    "orange-red" |
    "salmon" |
    "violet" |
    `custom-${string}`;

export function toRGB(color: Color, alpha: number = 1) {
    if (color.startsWith("custom-")) {
        return color.substring(color.indexOf("-") + 1);
    }        
    else { 
        return `rgba(var(--${color}-chalk), ${alpha})`;
    }
}