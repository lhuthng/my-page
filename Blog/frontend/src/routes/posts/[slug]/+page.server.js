import { mediaSyntax } from "$lib/common.js";
import { mediaWithShortcutPlugin } from "$lib/custom-rules/index.js";
import { fixClientRoute, route } from "$lib/server/proxy.js";
import { error } from "@sveltejs/kit";
import MarkdownIt from "markdown-it";

export async function load(event) {
    const res = await fetch(route(`posts/${event.params.slug}`), {
        method: "GET",
    });

    if (res.ok) {
        const data = await res.json();

        let { content, medium_urls, ...rest } = data;

        let edits = [...content.matchAll(mediaSyntax)].map((match) => ({
            index: match.index + match[0].lastIndexOf(match[1]),
            length: match[1].length,
            replacement: medium_urls[parseInt(match[1])],
        }));

        edits.sort((a, b) => b.index - a.index);

        const mediaDictionary = {};

        medium_urls.forEach(
            (url, index) =>
                (mediaDictionary[index.toString()] = fixClientRoute(url)),
        );

        const md = new MarkdownIt().use(mediaWithShortcutPlugin, {
            mediaDictionary,
        });

        content = md.render(content);

        return { content, medium_urls, ...rest };
    }

    throw error("Error", 404);
}
