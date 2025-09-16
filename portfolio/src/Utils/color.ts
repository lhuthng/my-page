export type Color = 
    "white" |
    "black" |
    "dark-charcoal" |
    "yellow" |
    "orange" |
    "blue" |
    "cyan" |
    "navi" |
    "green" |
    "aquamarine" |
    "lime" |
    "pink" |
    "purple" |
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