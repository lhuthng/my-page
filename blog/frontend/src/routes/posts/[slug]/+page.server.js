import { mediaSyntax } from "$lib/common.js";
import {
  appBlockPlugin,
  mediaWithShortcutPlugin,
  youtubeBlockPlugin,
} from "$lib/custom-rules/index.js";
import { fixClientRoute, route } from "$lib/server/proxy.js";
import { error } from "@sveltejs/kit";
import MarkdownIt from "markdown-it";
import mkKatex from "markdown-it-katex";

export async function load(event) {
  const res = await fetch(route(`posts/s/${event.params.slug}`), {
    method: "GET",
  });

  if (res.ok) {
    const data = await res.json();

    let { content, author_avatar_url, cover_url, medium_urls, ...rest } = data;

    let edits = [...content.matchAll(mediaSyntax)].map((match) => ({
      index: match.index + match[0].lastIndexOf(match[1]),
      length: match[1].length,
      replacement: medium_urls[parseInt(match[1])],
    }));

    edits.sort((a, b) => b.index - a.index);

    author_avatar_url = fixClientRoute(author_avatar_url);
    cover_url = fixClientRoute(cover_url);

    const mediaDictionary = {};

    medium_urls.forEach(
      (url, index) => (mediaDictionary[index.toString()] = fixClientRoute(url)),
    );

    const md = new MarkdownIt()
      .use(mkKatex)
      .use(mediaWithShortcutPlugin, { mediaDictionary })
      .use(youtubeBlockPlugin)
      .use(appBlockPlugin);

    content = md.render(content);

    return { content, author_avatar_url, cover_url, ...rest };
  } else {
    console.log(await res.text());
  }

  throw error(404, "Error");
}
