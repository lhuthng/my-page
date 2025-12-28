import { mediaSyntax } from "$lib/common.js";
import { fixClientRoute, route } from "$lib/server/proxy.js";
import { error } from "@sveltejs/kit";

export async function load(event) {
    const locals = await event.parent();

    const { post_id } = event.params;
    const { type, token } = locals.accessToken;

    const res = await event.fetch(route(`posts/id/${post_id}`), {
        method: "GET",
        headers: { Authorization: `${type} ${token}` },
    });

    if (!res.ok) {
        throw error(404, "Post Not Found");
    }

    const data = await res.json();

    data.medium_urls = data.medium_urls.map((url) => fixClientRoute(url));

    let { content, draft, medium_short_names } = data;

    let edits = [...content.matchAll(mediaSyntax)].map((match) => ({
        index: match.index + match[0].lastIndexOf(match[1]),
        length: match[1].length,
        replacement: medium_short_names[parseInt(match[1])],
    }));

    edits.sort((a, b) => b.index - a.index);

    edits.forEach(({ index, length, replacement }) => {
        content =
            content.slice(0, index) +
            replacement +
            content.slice(index + length);
    });

    data.content = content;

    edits = [...draft.matchAll(mediaSyntax)].map((match) => ({
        index: match.index + match[0].lastIndexOf(match[1]),
        length: match[1].length,
        replacement: medium_short_names[parseInt(match[1])],
    }));

    edits.sort((a, b) => b.index - a.index);

    edits.forEach(({ index, length, replacement }) => {
        draft =
            draft.slice(0, index) + replacement + draft.slice(index + length);
    });

    data.draft = draft;

    return { ...data };
}
