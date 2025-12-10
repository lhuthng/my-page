const styles = getComputedStyle(document.documentElement);
let cssRawCache: { [key: string]: string } = {};

export function GetVariable(name: string): string {
    if (cssRawCache[name] === undefined) {
        cssRawCache[name] = styles.getPropertyValue(name).trim();
    }
    return cssRawCache[name]
}

export function GetColor(name: string, normal: boolean = true): number[] {
    return GetVariable(name).split(', ').map(v => parseInt(v.trim(), 10) / (normal ? 256 : 1));
}

export function Clear() {
    cssRawCache = {}
}