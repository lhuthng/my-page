# v86 saves: adding a registry-based save method

Status: proposal
Scope: `blog/backend` + `blog/frontend` + `blog/backend/assets/v86/windows9x`

## Problem

Today a v86 game's progress is captured by a manifest key that lists **file
paths** on the game drive (`save_paths`). The in-guest launcher mirrors those
files onto the save floppy. That works for games that write a `.SAV` somewhere
under `D:\`.

It does not work for Windows 9x games that keep their settings and progress in
the **Windows Registry** (`HKEY_CURRENT_USER\Software\...`) and never write a
save file at all. Those games currently report `save_supported: false`, so the
Save button is hidden and progress is lost on every reload.

This document is the plan for a second save method alongside the existing one.

## What the current system actually is

Worth stating plainly, because it makes the new method cheap: **the save is a
single 1.44 MB floppy image.** Everything else is plumbing around that one fact.

| Layer | File | What it does |
| --- | --- | --- |
| Manifest schema | `blog/frontend/src/lib/features/v86/manifest.js:177` | `saveFilesFromManifest()` reads `save_paths` / `save_path` / `saves` |
| Server validation | `blog/backend/src/infrastructure/web/api/handlers/v86.rs:758` | `save_files_from_manifest()` validates the same keys |
| Feature gate | `v86.rs:2404`, `v86.rs:2593` | `save_supported = has_save_paths(manifest)` |
| Config emit | `manifest.js:201` | `launcherConfigFor()` writes `[game]` + `[saves] file=...` |
| Config baking | `blog/frontend/src/lib/features/v86/build-game.js:250` | `V86GAME.INI` + `LAUNCHER.EXE` are baked into each variant's launcher ISO |
| Guest side | `blog/backend/assets/v86/windows9x/game_launcher.c` | parses `[saves]`, mirrors `D:\` -> `A:\` every 2 s, restores `A:\` -> `D:\` on boot |
| Transport | `blog/frontend/src/lib/players/V86Player.svelte.js` | `get_disk_fda()` to read, `set_fda({buffer})` to mount |
| Storage | `blog/frontend/src/lib/players/v86-saves.js` | zstd blob; cloud `game_v86_saves` when logged in, IndexedDB otherwise |

Drive letters: `C:` = shared immutable Windows base, `D:` = per-session game
disk rebuilt from an immutable artifact on every boot, `A:` = the save floppy,
`E:` = the launcher CD.

`D:` being rebuilt on every boot is exactly why `A:` exists. The floppy is the
only thing that survives a page reload.

## Why the registry cannot be treated as one more file path

Two facts rule out the obvious approach of adding `USER.DAT` to `save_paths`:

1. `C:\WINDOWS\USER.DAT` (HKCU) and `C:\WINDOWS\SYSTEM.DAT` (HKLM, HKCR,
   `HKU\.DEFAULT`) are opened exclusively by Windows for the whole session.
   `CopyFileA` against them from inside Windows fails or yields a torn hive.
2. A game writes its keys at moments we do not control, usually on a graceful
   exit, and `D:` cannot hold them across sessions anyway.

The supported route in is `REGEDIT.EXE`, which ships with Windows 9x and needs
no elevation (Win9x has none):

```bat
REGEDIT /E A:\V86REG1.REG "HKEY_CURRENT_USER\Software\Acme\Doomlike"
REGEDIT /S A:\V86REG1.REG
```

`/E` exports a branch to a `REGEDIT4` ANSI text file, `/S` imports silently.
Both run happily while Windows is up. One game branch is a few KB, so the 1.44 MB
floppy is never the constraint, and **the registry rides the existing floppy** —
which means no new table, no new endpoint, no new storage key, no new client
state machine.

## Design

### 1. Manifest surface

New key, same shape and spirit as `save_paths`:

```ini
exe=GAME.EXE
save_paths=SAVE0001.DAT
save_registry=HKEY_CURRENT_USER\Software\Acme\Doomlike, HKEY_LOCAL_MACHINE\Software\Acme\Doomlike
```

Aliases to accept: `save_registry`, `save_reg_keys`, `save_keys`.
Separators: `,` or `;`, same as `save_paths`.

Validation rules (must be identical in Rust and JS — the two already mirror each
other byte for byte, keep that property):

- must begin with `HKEY_CURRENT_USER`, `HKEY_LOCAL_MACHINE`, `HKEY_USERS` or
  `HKEY_CLASSES_ROOT`, or the short forms `HKCU`, `HKLM`, `HKU`, `HKCR`
  (case-insensitive)
- at most 8 keys, at most 200 characters each
- reject `,` `;` `=` `"` and control characters inside a key
- reject a leading `\`, `..`, `.` components, and any `A:` / `C:` / `D:` drive
  prefix — this is a registry path, never a file path

One `.REG` file per key, because `/E` takes exactly one key per invocation.
Reserve the names `V86REG1.REG` .. `V86REG8.REG` at the floppy root, and share
that constant across the three layers so they cannot drift.

### 2. Guest launcher (`game_launcher.c`)

New section in `V86GAME.INI`, emitted after `[saves]`:

```ini
[registry]
key=HKEY_CURRENT_USER\Software\Acme\Doomlike
key=HKEY_LOCAL_MACHINE\Software\Acme\Doomlike
```

Changes, all local to the existing structure:

1. `parse_registry_keys()` — same double-NUL section walk as `parse_save_files()`
   (`game_launcher.c:407`), with `MAX_REG_KEYS 8` and `MAX_REG_KEY 200`.
2. `export_registry()` — per key, `CreateProcessA` on
   `C:\WINDOWS\REGEDIT.EXE` with `/E "A:\V86REGn.REG" "<key>"`, then wait on the
   process. Set `STARTF_USESHOWWINDOW` + `SW_HIDE` so no window flashes. Fall
   back to a bare `REGEDIT.EXE` if the absolute path is missing. Skip silently
   when `A:\` is not reachable — the key may also legitimately not exist yet on
   a first run, which is a normal `/E` failure, not an error.
3. `import_registry()` — per key, if `A:\V86REGn.REG` exists and starts with
   `REGEDIT4` (or `Windows Registry Editor`), run `REGEDIT /S`. It must run
   **before** `CreateProcess(game)`, and behind the same `floppy_reachable()`
   retry that the file restore uses, so the snapshot-restore race documented at
   `game_launcher.c:366` is handled identically.
4. Mirror thread (`mirror_thread()`, `game_launcher.c:384`): after the existing
   `mirror_all_saves()` poll, export the registry on a slower cadence — roughly
   every 15th tick, about 30 s, so the floppy is not rewritten every 2 s. Also
   export once immediately after `WaitForSingleObject(process.hProcess,
   INFINITE)` next to the final `mirror_all_saves()` call.
5. `g_save_file_count > 0` becomes `g_save_file_count > 0 || g_reg_key_count > 0`
   in the two places that gate the mirror thread and arm `g_restore_pending`.

Two behaviours to document in the code, because they are surprising:

- The import has to land before the game starts. A running game will not re-read
  the hive, so a mid-session import is useless.
- If the visitor closes the tab without exiting the game, and the game only
  writes its keys on a graceful exit, those keys are lost. The 30 s polling
  export is the mitigation. See "If you meant CPU registers instead" below for
  the only complete answer.

Rebuild with `blog/backend/assets/v86/windows9x/build-launcher.sh` (needs
`mingw-w64`; `-nostdlib -nostartfiles -fno-builtin -mno-stack-arg-probe
-mwindows -lkernel32 -luser32`) and commit the new `LAUNCHER.EXE`.

### 3. Client manifest layer

- `manifest.js`: add `registryKeysFromManifest()`, mirroring the Rust validator,
  and extend `launcherConfigFor()` (`manifest.js:201`) to append `[registry]`
  after `[saves]`.
- `manifest-editor.js`: add `REGISTRY_ALIASES` next to `SAVE_ALIASES`
  (`manifest-editor.js:42`), a `splitRegistryKeys()`, `model.registryKeys`, and a
  serialize line next to the existing `save_paths=` emit
  (`manifest-editor.js:133`). This preserves the parse -> edit -> serialize ->
  parse round-trip the editor is built around.
- `ManifestEditor.svelte`: clone the "Save paths" list editor
  (`ManifestEditor.svelte:167`) into a "Registry keys" block, plus a hint:
  *open `regedit`, find the branch the game writes under
  `HKEY_CURRENT_USER\Software` or `HKEY_LOCAL_MACHINE\Software`, and export a
  `.reg` to confirm it.*
- `build-game.js`: no change needed, it already calls `launcherConfigFor()`.

### 4. Backend

Contained, no migration:

- `v86.rs`: `validate_registry_key()` + `registry_keys_from_manifest()` next to
  `save_files_from_manifest()` (`v86.rs:758`); `has_save_registry()` next to
  `has_save_paths()` (`v86.rs:2593`).
- `v86.rs:2404`: `save_supported = has_save_paths(m) || has_save_registry(m)`.
- Add a `save_mode` field to the public runtime descriptor next to
  `save_supported` (`v86.rs:337`, `v86.rs:2583`), one of `"files"`,
  `"registry"`, `"files+registry"`, so the UI can phrase things correctly.
- Unit tests next to `save_files_parse_and_validate` (`v86.rs:3707`).

Unchanged on purpose: `game_v86_saves`, the `v86/saves/{user_id}/{game_id}/save.zst`
storage key (`v86.rs:3617`), the 2 MiB upload cap, the 30 s rate limit, and the
proxy route `blog/frontend/src/routes/api/projects/s/[slug]/v86/saves/+server.js`.

### 5. Player UI

- `V86Player.svelte.js`: `saveAvailable` already reads `save_supported`
  (`V86Player.svelte.js:75`). Add a derived `saveMode` and vary the copy: in
  registry mode, `saveMessage` should say progress was saved *and* the panel
  should carry one line of advice — *quit the game to its menu before saving, so
  it flushes its settings to the registry.*
- The 30 s cooldown, the `0x55AA` boot-signature validity check and the
  `get_disk_fda()` read in `saveNow()` (`V86Player.svelte.js:546`) all stay as
  they are. Nothing about the transport changes.
- The game page (`blog/frontend/src/routes/games/[slug]/+page.svelte`) needs no
  structural change; the `instruction` field is already where admins put
  per-game advice.

## If you meant CPU registers, not the registry

If the games in question keep progress in RAM and CPU state and never write to
disk or the hive, the fix is a **per-user machine snapshot**, not a floppy.
`V86Player.captureState()` (`V86Player.svelte.js:429`) already wraps
`emulator.save_state()`. But it is a different project:

- the blob is RAM plus disk overlay, tens of MB rather than 1.4 MB, so it needs
  its own storage key, size cap and rate limit;
- it is bound to `V86_TOPOLOGY_VERSION` (`V86Player.svelte.js:18`) and to the
  exact base and game disk SHAs, so any base image or device-list change
  invalidates every user snapshot at once;
- it restores the whole machine, which fights the current design where the
  launcher's delay and deferred floppy attach exist precisely so a restored
  state still picks up the visitor's own save.

Recommend doing the registry path first. It is small, it reuses the floppy, and
it does not invalidate anything.

## Rollout gotcha: the launcher is baked into every game's ISO

`LAUNCHER.EXE` is served from `blog/backend/assets/v86/windows9x/` and baked
into each variant's launcher ISO **by the client** at build time
(`build-game.js:249`), addressed by `iso_sha256`. Shipping a new launcher does
**not** retro-fix existing games.

- Each game's ISO has to be rebuilt. There is already a path for it:
  `buildLauncherIsos()` (`build-game.js:276`) rebuilds the launcher CDs while
  reusing the stored game disk, which is exactly what a manifest-only change
  does.
- Consider exposing the launcher's SHA or a version number in the runtime
  descriptor so the studio can list "games still on an old launcher" instead of
  discovering it by testing.

## Test plan

1. Rust unit tests: valid and invalid keys, every alias, the 8-key cap, and
   `save_supported == true` for a registry-only manifest.
2. JS unit tests: `manifest-editor.test.js` round-trip for `registryKeys`;
   `manifest.js` -> `launcherConfigFor()` output compared byte for byte against
   an expected `[registry]` block.
3. Manual round-trip in the sandbox page: boot Win98, open `regedit`, create
   `HKCU\Software\Test\Game` with a value, click Save, reload, confirm the value
   is back.
4. Automated round-trip: a manifest with one key, a tiny program that writes it,
   save / reload / verify.
5. Negative: registry-only manifest under a snapshot, where `A:` is attached
   late — the retry must still land the import.
6. Regression: a `save_paths`-only game must produce a byte-identical
   `V86GAME.INI` to today.

## Suggested order

1. Rust and JS validators plus unit tests. No runtime effect yet.
2. `game_launcher.c`: `[registry]` parse and import-before-launch. Rebuild and
   commit `LAUNCHER.EXE`.
3. Export path, mirror-thread cadence, post-exit export.
4. Manifest editor UI.
5. Runtime descriptor `save_mode` plus player copy.
6. Manual sandbox round-trip, then rebuild ISOs for the games being enabled.
