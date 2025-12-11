export function preventDefault(e) {
    e.preventDefault();
}

export const mediaSyntax = /\@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;

export function textToDate(text) {
    const dt = new Date(text.split(" ")[0]);
    const options = { year: "numeric", month: "short", day: "numeric" };
    return dt.toLocaleDateString("en-US", options);
}

export function nowToDate() {
    const dt = new Date();
    const options = { year: "numeric", month: "short", day: "numeric" };
    return dt.toLocaleDateString("en-US", options);
}
