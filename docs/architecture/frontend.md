# Frontend Architecture

Audience: frontend contributors. Update trigger: a change to the request
paths, the adapter, or the SSR setup.

The frontend is a fully server-side rendered SvelteKit 2 app running as a
persistent Bun process (`svelte-adapter-bun`); there is no static export. In
production it is containerized (`FROM oven/bun:1`) behind nginx.

## Structure

- `src/routes` — page routes, `+page.server.js` loads, and the `/api/[...path]`
  proxy route.
- `src/lib/server/proxy.js` — `route()` (server-side API calls) and
  `fixClientRoute()` (browser media URLs).
- `src/lib/components` — UI building blocks, including the editor shell,
  dashboard widgets, and the audiobook player.
- `src/lib/api` — typed wrappers around the REST surface.

## Persona roles

The site is written by one person and published under three accounts, listed in
`$lib/config/personas.js`: `systems` (The Architect), `personal` (Thắng), and
`creative` (Memory Field). `personaFor(slug)` resolves a profile slug to one of
them, and `roleFor(slug)` to the value a component should put on the DOM.

Any surface that belongs to a persona carries `data-role`, and its colour comes
from there rather than from a class:

```text
the component says:   data-role={persona?.role}
                      class="role-surface hover:role-surface-hover text-role-text"

app.css decides:       [data-role='personal'] { --color-role-surface: ...; }
                      (and the same block for systems and creative)
```

So the blue, green, and orange ramps are declared once in `app.css` and every
card, panel, and profile inherits them: a persona's post card is tinted, its
title, byline, excerpt, and tags take the dark end of the ramp, the expand
triangle and the excerpt panel's scrollbar take the base colour, and the black
panels use `role-on-dark` for the light end. There is no badge, because the
surface itself says who wrote it. An account that is not a persona -- a reader
who signed up to comment -- has no `data-role` and keeps the neutral default,
which is the site palette.

Not every tinted surface takes an outline. The panels -- the persona cards on
the homepage's black panel, the post-page author panel, the profile header --
carry `border-role-border` at `border-3`, because a floating panel with nothing
to separate it from the page behind dissolves into the background. Post and
project cards deliberately do not: they already have a drop shadow and a 3px
thumbnail frame, and an edge around the body read as a second competing border
rather than as the card's own.

Reading time is the exception. `.reading-cover` sets its own `--awareness-*`
colours, and by default a thumbnail frame and its reading bar wear the tier
colour, so length can be read at a glance without reading the number. On a
persona card that competes with the persona -- green is both the personal ramp
and the shortest tier -- so every post and project card opts in with
`role-outline`, which repaints both the frame and the bar in the author's hue.
The minutes are still printed on the bar, so length stays legible; what is lost
is the ability to compare lengths by colour, which is a fair trade for colour
meaning one thing only.

That exception also constrains the palette. The three personas take blue,
green, and orange: two warm hues at full saturation (red and orange) read as an
error state next to each other, and red is already the site's alert colour, so
it cannot double as an identity. Green is the one ramp that overlaps a
reading-time tier, which is why `role-outline` matters most there -- without it
a short post by Thắng would carry a green bar inside a green frame and the
length would be indistinguishable from the author.

## Request flow

```text
1. server-side data fetching
     hooks.server.js, +page.server.js
       `-- route() in $lib/server/proxy.js
             `-- prepends API_URL -> http://backend:3000
                 (Docker-internal, never public)

2. browser API calls
     fetch('/api/<path>')
       `-- src/routes/api/[...path]/+server.js
             `-- proxyFallback() -> API_URL (the backend)

3. media files
     fixClientRoute() in $lib/server/proxy.js
       |-- BACKEND_ORIGIN set   -> the browser fetches that origin directly
       |                           (nginx routes /media/* straight to the
       |                            backend, skipping SvelteKit)
       `-- BACKEND_ORIGIN unset -> /api/media/... (proxy fallback)
```

The media split keeps one network hop off the frontend container and lets the
backend's `Cache-Control` headers reach the browser unmodified.

## Audiobook chapters

The player never receives a book's whole chapter list. The detail load asks for
one window (`?tracks_offset=0&tracks_limit=20`) and gets `track_count` and
`has_more_tracks` back alongside it; the browser fetches later windows through
the same path as the reader reaches them — including, for a book left
mid-way through, the window the saved position sits in.
`src/lib/players/chapter-windows.js` holds the window arithmetic and
`AudiobookChapters.svelte.js` the fetching and the slot plan the list renders.
That plan is chapters plus placeholders: one row per window still missing, which
is the row that scrolls into view and fetches its own window. The placeholders
have to stay in the list in every order — filter them out and the book is
silently truncated at whatever the first fetch returned, with nothing left to
reach the rest. Searching and the reversed chapter order are the two cases that
ask for every remaining window at once, because both are browsing the book as a
whole.

The macOS player (`apps/macos`) uses the same window and the same arithmetic, so
both clients ask the backend for the same amount per request.

## Environment

See [../guides/configuration.md](../guides/configuration.md) for `API_URL`,
`BACKEND_ORIGIN`, `ALLOWED_HOSTS`, `TRUSTED_ORIGINS`, `PORT`, and
`BODY_SIZE_LIMIT`. Build output goes to `build/` with entry point
`build/index.js`.
