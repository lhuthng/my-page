export function ChangeColor(
    json: any,
    path: string,
    color: number[]
) {
    try {
        const names = path.trim().split('.');
        if (names.length < 3) return false;
        const layer = json?.layers?.find((layer: any) => layer?.nm === names[0]);
        const shape = layer?.shapes?.find((shape: any) => shape?.nm === names[1]);
        let current = shape;
        for (let depth = 2; current && depth < names.length; depth++) {
            current = current?.it?.find((item: any) => item?.nm === names[depth]);
        }
        const c = current?.c;
        if (c && c?.k) {
            c.k = color;
            return true;
        }
        return false;
    } catch (error) {
        return false;
    }
}

export function GetNamesWithColor(
    json: any
): any[] {
    const result: any[] = [];
    try {
        const path: string[] = [];

        const recursion = (parent: any) => {
            path.push(parent?.nm);
            switch (parent?.ty) {
                case "st": 
                    result.push({
                        path: path.join('.'),
                        type: "stroke",
                        color: parent.c
                    });
                    break;
                case "fl": {
                    result.push({
                        path: path.join('.'),
                        type: "fill",
                        color: parent.c
                    });
                    break;
                }
                case "gr": {
                    parent?.it?.forEach((child: any) => recursion(child));
                }
            }
            path.pop();
        }

        json?.layers?.forEach((layer: any) => {
            path.push(layer?.nm);
            layer?.shapes?.forEach((shape: any) => recursion(shape));
            path.pop();
        });
    } catch (error) {
        console.warn(error);
        return [];
    }
    return result;
}