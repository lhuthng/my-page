# Editor feedback UX — design spec

Status: **implemented** (2026-09-11). See §9 for what landed and where it
departs from this document.
Scope: the post / project / game editor under `frontend/src/lib/components/editor/`.

## 1. The problem

Editor feedback currently has **no lifetime model**. Every message — a save
confirmation, a validation error, upload progress, a media warning — is written
into the same two fields:

```js
ui.notice          // a single string slot
ui.noticeCritical  // a boolean that only picks the colour
```

Because there is one slot and no shared rule for how long a message lives, the
lifetime of a message is an accident of the code path that produced it. In
practice:

| Message | Today's lifetime | Exit |
| --- | --- | --- |
| `OK!`, `Published!`, `Nothing to save.` | 2200 ms (`NOTICE_MS`) | time |
| Validation error, `Save failed.`, conflict | forever (`autoClearMs: 0`) | **none** |
| Upload progress (`ui.progress`) | forever — never reset to `''` | **none** |
| Slug `checking… / ok / taken` | per-slug map entry, never cleared | **none** |
| Missing-media warning | appears/vanishes on the 500 ms preview tick | condition, but flickers |

Three concrete consequences:

1. **Stale errors are permanent.** After one failed save, the red line stays in
   the toolbar for the rest of the session. Nothing clears it except a later
   `notify()` call happening to overwrite it. There is no dismiss control.
2. **Progress outlives the work.** `setProgress` only ever writes; no code path
   resets `ui.progress` to `''`. "Building launcher CD — part 3 of 8" remains
   on screen after the upload has finished.
3. **Messages cannibalise each other.** A success toast and an upload progress
   line share one slot, so whichever fired last wins and the other disappears
   mid-read.

The debounce is not the root cause — it only makes the problem visible, because
the two debounce-driven messages are the ones whose *appearance* is timed
rather than caused.

## 2. The model

Replace "notice + critical flag" with three classes. **The trigger decides the
class, not the severity.**

### Class 1 — live (inline status)

Continuous state that describes something the editor already knows. It is
**derived**, never posted, and it disappears when the condition stops being
true. No dismiss control, no timer.

Use for: saved/unsaved, slug availability, upload/build progress, media
resolution, character counts.

> Rule: if the message is a function of editor state, it is live. It must never
> be pushed into a toast.

### Class 2 — sticky (dismissible banner)

Something that blocks the user's intent and needs a decision. Persists until
**one of three** things happens: the user resolves it, the user dismisses it
with `×`, or a retry succeeds. Always has an explicit `×`.

Use for: save conflict, save failure, upload failure, "a locally-saved draft is
newer" recovery prompt.

> Rule: a sticky message without a `×` is a bug.

### Class 3 — transient (toast)

The direct acknowledgement of a user action. Auto-dismisses after **4000 ms**,
pauses its timer on hover, and carries a `×` for impatience. Rendered in a
floating stack so it never shifts layout.

Use for: "Saved", "Published", "Draft created".

> Rule: a transient message must never be the only report of a failure.

### The exit invariant

> **Every message must have at least one exit: time, resolution, or an explicit
> dismiss. A message with none of the three may not ship.**

This single rule is the fix. It is enforceable in review by asking "how does
this one go away?"

## 3. Component contract

One small store, three render regions.

```js
// feedback.svelte.js — sketch
createFeedback({
  toast(message, { tone = 'success', ms = 4000 }),   // class 3
  banner(id, message, { tone, actions, onDismiss }),  // class 2, keyed by id
  dismiss(id),                                        // class 2 + 3
  live(key, value)                                    // class 1, set or clear
})
```

- `banner` is **keyed by id**, so re-reporting the same failure refreshes the
  existing banner instead of stacking a duplicate.
- `live(key, null)` clears the slot — this is the missing call that currently
  lets `ui.progress` leak.
- `toast` uses a bounded stack (max 3); the oldest is evicted.

### Render regions

| Region | Class | Position | Layout impact |
| --- | --- | --- | --- |
| Toolbar status line | live | inside the sticky header, below the title row | none (fixed height) |
| Banner stack | sticky | below the header, above the body | pushes body down |
| Toast stack | transient | floating, bottom-right of the editor | none (`position: absolute`) |

Toasts float so that a save confirmation never reflows the editor. Banners
push, because a blocking problem deserves the space.

## 4. Message-by-message migration

| Today | Becomes | Class |
| --- | --- | --- |
| `notify('OK!')` | toast "Saved" / "Published" / "Draft created" | transient |
| `notify('Nothing to save.')` | toast, tone `neutral` | transient |
| `notify(basicsError, {critical})` | **inline field error** under the offending input + focus it | live |
| `notify('Save failed.')` | banner with `Retry` + `×` | sticky |
| `notify('Someone else saved this…')` | keep the existing conflict banner, add `×` | sticky |
| `notify('[x] is/are missing')` | merge into the live media indicator | live |
| `ui.progress` | live slot, cleared on completion | live |
| `ui.slugStatus[slug]` | live slot, cleared when slug changes | live |
| `missingKeys` banner | live counter beside the "Media library" heading | live |
| `localDraftAvailable` banner | keep, add `×` (= discard) | sticky |

Validation errors are the biggest change: they should not be a toolbar banner
at all. A red line at the top of the page does not tell you *which* field is
wrong. Move them to the field.

## 5. The debounce-specific fix

The debounce timers themselves are correct — 300/500/800 ms on slug, preview,
and autosave is reasonable. The bug is that a *debounced* tick is allowed to
raise a *message*.

Two changes:

**5.1 Missing media becomes live, not a message.** It renders as a counter next
to the "Media library" heading: `4 of 4 media resolved` → `2 missing`. It never
flashes, because a token only enters the count once it is *settled* — a
completed reference, not a word you are still typing. Implement by requiring
the token to be terminated (whitespace, punctuation, or end of input) before it
is evaluated, in addition to the existing 500 ms debounce.

**5.2 Preview staleness gets a live signal.** Today the preview pane silently
catches up 500 ms after you stop typing. Add a small live state on the preview
header: `Preview` → `Preview · updating` while the debounce is pending. This
makes the lag legible instead of looking like dropped input.

## 6. Accessibility

- Toasts and banners render in `role="status"` (polite) for success and
  `role="alert"` (assertive) for errors.
- Sticky banners must be reachable by keyboard; `×` needs an
  `aria-label="Dismiss"`.
- The live status line uses `aria-live="polite"` with `aria-atomic="false"` so
  only the changed fragment is announced.
- The `×` must never be the only way to resolve a blocker — every sticky banner
  also offers a primary action.

## 7. Implementation touchpoints

- `frontend/src/lib/features/editor/view-model/create-editor-vm.svelte.js`
  — replace `notify()` / `ui.notice` / `ui.noticeCritical` / `ui.progress` /
  `ui.slugStatus` with the feedback store. `NOTICE_MS` and the `autoClearMs: 0`
  call sites (roughly 20 of them) all collapse into the three methods.
- `frontend/src/lib/components/editor/EditorToolbar.svelte`
  — currently renders `ui.progress`, `ui.notice`, the conflict banner, and the
  local-draft banner inline in one column. Split into live row + banner stack.
- `frontend/src/lib/components/editor/PostEditorShell.svelte`
  — `missingKeys` moves from a red paragraph to a live counter; add the preview
  staleness signal.
- `frontend/src/lib/features/editor/data/local-draft.js`
  — unchanged; the recovery banner just gains a `×`.

## 8. Suggested sequencing

1. Introduce the feedback store and route the **sticky** messages through it
   with a `×`. This alone removes the permanent-stale-error bug.
2. Move `ui.progress` and slug status to **live** slots and clear them.
3. Convert success messages to **toasts**.
4. Move validation errors **inline** to their fields.
5. Re-tune the media indicator (settled tokens) and add the preview signal.

## 9. Implementation record

Shipped 2026-09-11. Files: `view-model/feedback.svelte.js` (the store),
`model/feedback.js` (the pure helpers it uses, testable from plain Node),
`components/editor/EditorFeedback.svelte` (both stacks), plus edits to
`create-editor-vm.svelte.js`, `EditorToolbar.svelte`, `PostEditorShell.svelte`,
`ContentDebounceEditor.svelte`, `ProjectEditor.svelte`, `GameEditor.svelte`,
`model/validate.js`.

### Where it departs from this document, and why

**Toasts are `fixed`, not `absolute`.** The spec says "bottom-right of the
editor, `position: absolute`". On a long post an absolute toast would anchor to
the bottom of the article and scroll out of view, which defeats the purpose of
an acknowledgement. It floats over the viewport instead — still no layout
impact, which was the actual requirement.

**A blocked save raises a banner as well as updating the live counter.** §4 says
missing media should "merge into the live media indicator", and it does — the
counter beside the Media library heading. But a save that is silently refused
while the explanation sits elsewhere on the page is worse than either. So the
refusal also raises a sticky banner keyed `missing-media`, listing the names,
with its exit being resolution: it clears itself as soon as the references
resolve (or via `×`).

**No extra "settled token" gate was added.** §5.1 asks for tokens to be
terminated before evaluation. The collect patterns in
`media/references.js` already require exactly that: `@[img:key]` needs its
closing `]`, and `:::app glb-demo key` needs a whitespace-delimited token. A
half-typed `@[img:my-` cannot match. Adding a further trailing-terminator rule
would suppress the indicator for a reference typed at the end of the body, which
is the common case — worse, not better. The flashing came from the *paragraph*
appearing and disappearing, and the counter removes that.

**Field errors clear on `input`, not from a derived.** A `$derived` would have
to read and write the same slot, so the clearing is an explicit `oninput` →
`clearFieldError(field)`. Predictable, and no effect-loop to reason about.

**Two more messages joined the model.** Project and game *delete* failures were
writing `vm.ui.notice` and `vm.ui.noticeCritical` directly from the component.
They are now sticky banners with `Retry` and `×`, so they have an exit like
everything else.

### The exit inventory

Every message the editor can now emit, and how it goes away. This is the table
to check a new message against before adding it.

| Message | Class | Exit |
| --- | --- | --- |
| Saved / Published / Draft created | toast | time (4 s), hover pause, `×` |
| Nothing to save. | toast (neutral) | time, `×` |
| Title / Slug / Excerpt / demo invalid | live | resolution (editing the field) |
| Slug checking… / ok / taken | live | resolution (slug changes or lookup lands) |
| Upload / build progress | live | resolution (cleared on completion) |
| Media count (n missing) | live | resolution (references resolve) |
| Save failed / Publish failed | banner | `Retry`, `×` |
| Missing media on a blocked save | banner | resolution (auto-clears), `×` |
| Save conflict | banner | `Reload theirs` / `Overwrite mine`, `×` |
| Local draft is newer | banner | `Restore it`, `×` (= discard) |
| Delete failed (project / game) | banner | `Retry`, `×` |

No row is without an exit, which was the point.

### Known follow-ups

- The delete-failure banner is raised from the editor component, so it shares
  the editor's banner stack. If those dialogs ever move, the banner should move
  with them.
- `feedback.svelte.js` cannot be unit-tested from `node --test` (runes only
  exist after Svelte compiles a `.svelte.js` module), which is why the
  arithmetic and stack policy live in `model/feedback.js` with tests of their
  own. Behaviour of the store itself is only covered by the build and by use.
