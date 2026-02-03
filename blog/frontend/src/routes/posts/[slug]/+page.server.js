import { mediaSyntax } from "$lib/common.js";
import {
  appBlockPlugin,
  codeHighlightPlugin,
  mediaWithShortcutPlugin,
  namedContainerPlugin,
  revealPlugin,
  slugify,
  youtubeBlockPlugin,
} from "$lib/custom-rules/index.js";
import { fixClientRoute, route } from "$lib/server/proxy.js";
import { error } from "@sveltejs/kit";
import MarkdownIt from "markdown-it";
import mkKatex from "markdown-it-katex";
import anchor from "markdown-it-anchor";

export async function load({ fetch, params, setHeaders }) {
  const res = await fetch(route(`posts/s/${params.slug}`), {
    method: "GET",
  });

  if (res.ok) {
    setHeaders({
      "cache-control": "public, max-age=60, s-maxage=60",
    });
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
      .use(appBlockPlugin)
      .use(revealPlugin)
      .use(namedContainerPlugin)
      .use(codeHighlightPlugin)
      .use(anchor, { slugify });
    content = md.render(content);

    const series = rest.series;
    if (series) {
      series.cover_url = fixClientRoute(series.cover_url);
      const fixPost = (post) => {
        if (post) post.cover_url = fixClientRoute(post.cover_url);
      };
      fixPost(series.previous_post);
      fixPost(series.next_post);
    }

    return { content, author_avatar_url, cover_url, ...rest };
  } else {
    console.log(await res.text());
  }

  throw error(404, "Error");
}
