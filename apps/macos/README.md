# Audiobooks for macOS

A minimal native macOS app for browsing and playing the audiobooks served by
the blog backend (`blog/backend`). SwiftUI + AVFoundation, no third-party
dependencies, built with SwiftPM.

## Screens

- **Browser** — the catalogue: search, tag filter, and the site's white cards
  (wide 1.91:1 covers, chapter bar, VN flag chip) on the purple page wash,
  with the mini player pinned to the bottom while something is loaded.
- **Player** — picking a book slides the whole screen to the right. Extended
  player: large cover fading into the chapter list under a gradient veil, the
  mini player's green/red transport language, rate menu, and a labelled
  scrubber above the always-open chapter list, which fills in a window of
  chapters at a time. Browsing away from what is playing holds that chapter at
  the edge it left through — a flat bar with the app's own transport button,
  the elapsed time and a position rule; clicking it scrolls back to the
  playhead. Swipe back (or `Esc`, or the back chevron) returns to the browser.
- **About** — one page to the right of the browser (the header's ⓘ button, or a
  swipe): a branded card — the site's logo mark redrawn as vectors in white on
  the app's primary ramp, the name and version, what the app is, the blog's own
  tagline, the facts, and buttons out to the blog and the source. No mini player
  here until something is playing.

The three screens are pages of one strip with a seam between them; paging works
from the artwork, the transport row and the page wash, while a drag over the
chapter list or the timeline scrolls or scrubs and never turns a page. The strip
only reaches the player once a book is loaded. Chapter rows select on release,
and only when the press barely moved, so a swipe never also jumps chapter.

The browser's header is a **pull-down drawer**: it rests tucked above the top
edge behind a slim grip, and dragging the grip (or the title row) down slides
it in, pushing the catalogue down with it. The pull is rubber-banded — it can
be tugged a little past fully open and springs back on release — and clicking
the grip toggles it. The grip is drawn from the header's own measured height,
so the reveal is never guessed.

## Build & run

```sh
make run    # dev build, runs from the CLI
make open   # release build bundled as build/Audiobooks.app, then opens it
make test   # unit tests (the updater's payload, digest, and version logic)
```

A full Xcode install is required — the Command Line Tools alone cannot
compile SwiftUI (the macOS 27 CLT ships without the SwiftUI macro plugin).
The Makefile picks up `/Applications/Xcode.app` automatically; no
`xcode-select` needed. The package also opens directly in Xcode for IDE
iteration.

### Releasing

Pushing a `v*` tag builds the app on `macos-15` and attaches the zipped `.app`
to a GitHub Release (`.github/workflows/release.yml`):

```sh
git tag v0.1.0 && git push origin v0.1.0
```

The tag becomes the bundle's `CFBundleShortVersionString`, so bump
`AUDIOBOOKS_VERSION`'s default in `Scripts/bundle.sh` only if you want local
builds to differ from releases. The bundle is ad-hoc signed, not notarized, so
recipients right-click → Open on first launch. Publishing a tag is the only
step; nothing needs to be built by hand.

### Toolchain notes

`Scripts/toolchain-env.sh` puts the build on Xcode's toolchain and its macOS 27
SDK, which is the configuration this package is known to build cleanly in: no
linker warnings, and no need for the `DisableSwiftExplicitModules` workaround
that an earlier CLT-only setup required. Every `make` target runs through that
script, so `make debug`, `make app` and `make open` are all warning-free.

If Xcode is ever missing, the script falls back to the Homebrew swift.org
toolchain pinned against the CLT's macOS 26.5 SDK — that combination still
builds, but the linker reports three `search path ... not found` warnings and
four `linking with dylib ... built for newer version 27.0` warnings. Both are
noise from mixing the toolchain's and the CLT's layouts, not build problems, and
they disappear with Xcode installed.

### Deployment target

macOS 15. It is declared once in `Package.swift` (§`platforms`) and mirrored by
`LSMinimumSystemVersion` in `Scripts/bundle.sh` — change them together. The
package's manifest stays on tools 5.9, where the `v15` enum case does not
exist, hence the string form; the `.onChange(of:)` call sites use the
two-argument closure that macOS 14 introduced.

## Backend

Defaults to production (`https://api.huuthangle.site`). For local development
against the Axum backend:

```sh
AUDIOBOOKS_API_BASE=http://127.0.0.1:5174 make run
```

Used endpoints (all public, unauthenticated): `GET /audiobooks/public/all`
(`term`, `tag`, `limit`, `offset`, `has_more`), `GET /audiobooks/public/s/{slug}`
(`tracks_offset`, `tracks_limit`, answered with `track_count` and
`has_more_tracks`), and `GET /media/i/{short_name}` for streaming (HTTP Range).
The bundle's Info.plist allows plain HTTP to `localhost`/`127.0.0.1` so the dev
backend works from the bundled app too.

## Behavior notes

- Chapters are fetched a **window at a time** rather than with the book: the
  detail request asks for `tracks_limit` chapters from `tracks_offset` and the
  answer's `track_count` says how many the book really has, so the list can be
  drawn in full before the chapters are all here. `ChapterWindow` holds that
  arithmetic (20 per window, 3 ahead prefetched), and `ChapterStore` holds the
  chapters themselves. A book opened at chapter 300 therefore loads like one
  opened at chapter 1: the window holding the saved chapter is fetched on
  demand, and the transport waits for it instead of finding nothing to play.
- The chapter list renders a **slot per chapter** — a row where the chapter has
  arrived, that row's own shape in grey blocks where it has not. Slots are built
  lazily, and building one is what asks for its window, so scrolling is what
  loads the next window: a 2000-chapter book paints the rows near the viewport
  rather than all of them. The playhead keeps the next few chapters in hand, so
  a chapter boundary is not where the network shows up.
- Play counts coming back from a beacon are written into whichever chapter
  holds them — the loaded window on screen — rather than into the book's opening
  window, which is all `AudiobookDetails.tracks` holds for a windowed book.
- Progress (chapter, position, rate) is stored per book on device in
  `~/Library/Application Support/Audiobooks/progress.json`, mirroring the web
  player's localStorage payload. Reopening a book opens it **paused** at the point
  it reached, silent, and asks whether to carry on from there: nothing plays on
  its own, declining drops it back to the first chapter (still paused), and a book
  with no history simply opens at the top of chapter one, waiting for the play
  button.
- Covers are downsampled through ImageIO to ≤640px and cached in memory, so
  large artwork never sits fully decoded in RAM.
- System Now Playing integration (media keys, Control Center) comes from
  `MPNowPlayingInfoCenter` + `MPRemoteCommandCenter`: play/pause, previous /
  next chapter, ±15/30s skip, and position scrubbing.
- Track durations fall back to the asset's own duration when the backend has
  no `duration_seconds` for a chapter.
- The About page's version is read from the bundle (`CFBundleShortVersionString`,
  set by `Scripts/bundle.sh`), so it can't drift from the shipped bundle.
- Picking a book **navigates first and loads second**: the strip is sent to the
  player immediately (with the player put into its loading state, so the page
  has something honest on it as it slides in), and the fetch starts once the
  slide has finished. Loading first made the library's own mini player flip to
  the new book while the library was still the page on screen.
- Each press is read **once** as either a scroll or a page swipe, and only once
  one axis clearly leads the other — a press never changes its mind mid-gesture,
  because letting a scroll turn into a swipe (or back) had the list and the strip
  moving at the same time. The chapter list and the timeline own their presses
  outright: a drag over either scrolls or scrubs and never turns a page. Paging
  stays on the artwork, the transport row and the page wash around the card.
- The router **swallows the drags it drives**. A scroll view handed the same
  movement treats it as its own pan and reasserts its own offset on the next
  layout pass, snapping the list straight back — so a delivered drag-scroll reads
  as no scrolling at all. The press's scroll view is settled once, at mouse-down:
  the chapter list hands over its own `NSScrollView`, since SwiftUI's frames and
  the window's coordinates disagree about where the list is. Down and up are never
  swallowed, so taps, the timeline, the search field and buttons are untouched.
- A chapter row selects with a **tap**, not a Button and not a drag gesture: a
  tap recognizer is the one shape that hands the press back the moment the
  pointer moves, so a drag that starts on a row still reaches the scroll view.
- Scroll views in the app hide their indicators: with the system preference set
  to always show scroll bars, the browser's grid had a bar in a track of its own
  pushing the cards inward, and a white strip down the right edge.
- This window paints SwiftUI drawing but not the content of the AppKit-backed
  controls it hosts: a `Menu`'s own label and a text field's placeholder come out
  blank while still holding their layout space — the menu opens normally, and its
  dropdown (a window of its own) draws fine. So the header draws its own search
  placeholder and tag capsule on top of the controls, from SwiftUI.

## Roadmap ideas

- Offline downloads (media URLs are immutable, ETag = SHA-256)
- Cross-device progress sync once the backend grows an authenticated
  progress endpoint
- iOS target reusing Models / API / PlayerModel unchanged
