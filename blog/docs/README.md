# Design Proposals

Engineering artefacts for the blog: feature-level design documents. These do
not appear in the repository-wide `docs/` index.

| Document | Status | Scope |
| --- | --- | --- |
| [draft-body-collapse.md](draft-body-collapse.md) | proposal | Editor body model (`posts.draft` vs `content`) |
| [editor-feedback-ux.md](editor-feedback-ux.md) | implemented 2026-09-11 | Editor under `frontend/src/lib/components/editor/` |
| [v86-registry-saves-plan.md](v86-registry-saves-plan.md) | proposal | `blog/backend` + `blog/frontend` + `backend/assets/v86/windows9x` |

Rules for documents in this directory:

- Status front-matter right under the title, from the fixed vocabulary
  (`proposal`, `accepted`, `implemented`, `superseded`, `rejected`).
- A declared scope line naming the directories the document touches.
- Superseded documents are kept, not deleted, with a link to their
  replacement.

Repository-wide architecture decisions:
[../../docs/decisions/README.md](../../docs/decisions/README.md).
