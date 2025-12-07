export function preventDefault(e) {
    e.preventDefault();
}

export const mediaSyntax = /\@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]/g;
