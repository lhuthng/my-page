# Media and Storage

Audience: backend contributors and operators. Update trigger: a storage
layout or serving change.

## Where the bytes live

Every uploaded file — images, video, audio, models, Lottie — is written to
`MEDIA_PATH` and stays there. The one exception is **audiobook audio**, which
can *additionally* be stored in the R2 bucket and served from there.

That exception is the whole scope of the bucket migration. Covers, avatars,
post thumbnails, series art, video and models are disk-only and are not
mirrored, served, synced or backfilled through the bucket at all.

**"Audiobook audio" means one thing precisely: a `media` row that an
`audiobook_tracks` row points at.** That is an indexed foreign key, and it is
the definition the write path and `backfill-audio.sh` both use. Content type alone
would be too wide — it also matches a sound file uploaded to the media library
and the media of a track that has since been replaced, which
`replace_track_medium` deliberately leaves in place — and `media.description` is
free text the table fills with NULLs.

The read path cannot use that definition: it has only the row in front of it and
no join. It uses the content type instead, which is a superset. The superset is
inert — nothing but track audio is ever *put* in the bucket, so a media-library
mp3 has no object and the request falls through to the disk — and it buys a
property worth having: where a row is served from depends on the row, not on
which caller is asking.

## Media files

Media is content-addressed by hash + file type + uploader. The key is
store-independent: the disk path is `MEDIA_PATH.join(key)`.

| Row | `media.hash` | key |
| --- | --- | --- |
| regular media | `<sha256>` | `<sha[0..2]>/<sha[2..4]>/<sha256><ext>` |
| post cover | `.post.<post_id>.<sha256>` | `post/<uploader_id>/<sha256><ext>` |
| avatar | `.avt.<user_id>.<sha256>` | `avt/<uploader_id>/<sha256><ext>` |
| series cover | `.srs.<user_id>.<sha256>` | `srs/<uploader_id>/<sha256><ext>` |

For the three special layouts the id inside the hash is the post or user the
file is named after, while the directory is the *uploader* id. They are usually
equal and are not guaranteed to be, so `storage::media_key` is the only place
this is derived. `sync::canonical_media_url` delegates to it rather than
repeating the layout.

`storage::audio_object_key` adds the bucket prefix for audio and returns `None`
for everything else:

```text
audio, on disk   : MEDIA_PATH/b4/c0/<sha256>.mp3
audio, in bucket : retro-games/audio/b4/c0/<sha256>.mp3
an image         : MEDIA_PATH/b4/c0/<sha256>.webp   (disk only, forever)
```

Because the object key is the disk key under a prefix and nothing else, the
backfill is a plain copy and **no file has to move on disk**. The prefix exists
only because the bucket is shared with the v86 artifacts; it has no counterpart
on disk, because `MEDIA_PATH` holds nothing but media — unlike
`PROJECT_DEMOS_PATH`, which is shared with the demos, and is why `v86/` appears
in both stores.

Audio is served with HTTP Range requests (`206 Partial Content`), which is
what makes audiobook streaming seek instantly without loading a chapter into
memory.

## Audio in the bucket

| `AUDIO_BACKEND` | Behavior |
| --- | --- |
| `fs` (default) | Audio stays under `MEDIA_PATH`; the backend streams from disk. |
| `r2` | Each audio upload also goes to the bucket, and reads try the bucket before the disk. |

Connection details come from the `R2_*` variables — the same bucket as the v86
artifacts, told apart by the `audio/` prefix. There is deliberately no `auto`:
`STORAGE_BACKEND=auto` may pick R2 as a side effect of the credentials existing,
and audio moving to the bucket is a decision rather than something that should
happen because someone configured R2 for the artifacts. `AUDIO_BACKEND=r2`
requires `R2_PUBLIC_URL`, because that is where audio is served from.

- Reads are bucket-first with a disk fallback, so a tree that is only partly
  backfilled keeps serving. Anything that is not audio has no object key and
  falls straight through to the disk; audio that no track plays is not in the
  bucket either, so it does the same.
- `redirect` (default) answers `/media/i/{short_name}` with a 302 to the
  object's public URL, so the bytes never pass through the VM. Nothing is
  signed: the bucket has a public domain, so the URL is identical for every
  client and every request. `AUDIO_READ=proxy` streams through the backend
  instead, for debugging and for a client that mishandles redirects.
- The redirect is `no-store` even though the object is immutable. The object's
  identity is fixed, but the domain it is served from is configuration
  (`R2_PUBLIC_URL`), so a cached redirect would pin a client to a domain a later
  deploy may have moved off, for as long as the cache lives. The object itself
  carries `Cache-Control: public, max-age=31536000, immutable`.
- Audio is **public-by-URL** at `R2_PUBLIC_URL/audio/...`. The keys are sha256,
  so objects are unlisted rather than open — but that is not the same as
  private, and a prefix cannot be made private inside a public bucket without a
  Worker in front of it.
- Objects are stored with their real `Content-Type`. Audio served as
  `application/octet-stream` is refused by Safari and AVPlayer.
- `/media/s/{short_name}` returns `media/i/{short_name}` rather than the stored
  disk path, so the URL survives the bytes moving.
- Who takes the redirect matters. The macOS app and (in production, where
  `BACKEND_ORIGIN` is set) the blog's images fetch the backend directly and
  follow the 302 themselves, so the bytes go bucket → client. The blog's
  audiobook player does not: `fixUrl` gives `<audio>` a same-origin
  `/api/media/i/...`, the SvelteKit proxy follows the 302 server-side, and the
  first play still crosses the VM. Repeat plays come from the browser's
  one-year `immutable` cache.
- Nothing deletes an object. Replaced covers and avatars delete their old *file*
  on disk, which is all they ever did; audio is never replaced in place, because
  `replace_track_medium` deliberately keeps the old file so a track that shared
  its bytes with another one does not break. `backfill-audio.sh` lists any object
  nothing points at as "in bucket but no track".

Env vars: [../guides/configuration.md](../guides/configuration.md). Migration:
[../guides/operations.md](../guides/operations.md).

## ObjectStore (v86 game artifacts)

v86 artifacts — system IMG chunks, game disks, launcher ISOs, snapshots,
saves — go through the `ObjectStore` abstraction in
`src/infrastructure/storage/`. They share the bucket with the audio above and
live under the `v86/` prefix. HTML5/WebGL demo ZIPs and js-dos bundles always
live on local disk regardless of this setting.

| `STORAGE_BACKEND` | Behavior |
| --- | --- |
| `auto` (default) | Cloudflare R2 when the `R2_*` variables are set, otherwise the local filesystem |
| `r2` | Require R2; startup fails if the `R2_*` variables are incomplete |
| `fs` | Local filesystem under `PROJECT_DEMOS_PATH` |

- In `fs` mode artifacts land at `{PROJECT_DEMOS_PATH}/{storage_key}` — the
  same key layout the R2 mirror uses, so the backends are interchangeable.
- In `fs` mode the backend serves artifacts itself at relative URLs
  (`/v86/snapshots/...`, `/games/s/{slug}/v86/...`); `R2_PUBLIC_URL` is
  ignored, and nginx must route the artifact prefixes to the backend.
- In `r2` mode the descriptor points straight at the R2 public domain and the
  browser fetches chunks from the CDN.

Storage keys are content-addressed (sha256-derived), which makes the R2→fs
backfill idempotent — see [../guides/operations.md](../guides/operations.md).

Env vars: [../guides/configuration.md](../guides/configuration.md).
