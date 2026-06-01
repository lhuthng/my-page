export const COMMENT_COMMANDS = {
    KAOMOJI: 'kao',
    GIF: 'gif'
};

const commandPattern = /\/(kao|gif)(?:\s+(\S*))?$/;

export function getActiveCommentCommand(value, caret) {
    if (typeof value !== 'string') return null;

    const safeCaret = Number.isInteger(caret)
        ? Math.max(0, Math.min(caret, value.length))
        : value.length;

    const beforeCaret = value.slice(0, safeCaret);
    const lineStart = beforeCaret.lastIndexOf('\n') + 1;
    const activeLine = beforeCaret.slice(lineStart);
    const match = activeLine.match(commandPattern);

    if (!match) return null;

    const kind = match[1];
    const query = match[2] ?? '';
    const relativeStart = match.index ?? 0;
    const start = lineStart + relativeStart;

    // replaceEnd: extend to end of current word after caret (no paren needed)
    let replaceEnd = safeCaret;
    const afterCaret = value.slice(safeCaret);
    const nextBreak = afterCaret.search(/[\s\n]/);
    if (nextBreak !== -1) {
        replaceEnd = safeCaret + nextBreak;
    } else {
        replaceEnd = value.length;
    }

    return {
        kind,
        query,
        start,
        caret: safeCaret,
        replaceEnd
    };
}

export function replaceCommandRange(value, context, replacement) {
    if (!context) return value;
    return `${value.slice(0, context.start)}${replacement}${value.slice(context.replaceEnd)}`;
}

export function buildCommandToken(kind, query) {
    const normalizedKind = kind === COMMENT_COMMANDS.GIF
        ? COMMENT_COMMANDS.GIF
        : COMMENT_COMMANDS.KAOMOJI;
    const normalizedQuery = String(query ?? '');
    return normalizedQuery
        ? `/${normalizedKind} ${normalizedQuery}`
        : `/${normalizedKind} `;
}