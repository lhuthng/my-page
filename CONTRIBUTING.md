# Contributing

## Commits

Conventional commits with scopes, e.g. `feat(audiobooks): ...`,
`refactor(backend): ...`, `fix(editor): ...`, `docs(backend): ...`.
`[deploy blog]` in the message forces a deploy; `[manual deploy]` skips CI.

## Gates

- Backend: `cargo fmt` and `cargo clippy -- -D warnings` (both wired into
  `make lint`).
- Frontend: Prettier check (`make lint`), write with `make fmt`.
- Backend unit tests: `cargo test` from `blog/backend`.

## Migrations

- One concern per migration; ISO 8601 date prefix (matches `sqlx migrate`).
- Migrations run automatically on backend startup — a broken migration breaks
  the container. Table-rebuild migrations rely on the FK-off migration pool;
  see `docs/architecture/data-model.md`.

## Backend architecture rules

The layer model, dependency contract, and file budget live in
`docs/architecture/backend.md`. The short version:

- Dependencies point inward: `domain` ← `application` ← `infrastructure`.
- Handlers never touch `infrastructure::persistence` directly and never
  import other feature modules; shared handler utilities go in
  `web::api::support`.
- Route tables live in each feature's `routes()`; `router.rs` only composes.
- Production files stay within the file budget (≤ 400 lines hard ceiling);
  unit tests live in sibling `tests.rs` files.

## Documentation rules

Applies to every markdown file except third-party notices
(`blog/frontend/static/zstd/NOTICE.md`):

- One H1 per file; no skipped heading levels.
- Relative links only, and every link must resolve on a case-sensitive
  filesystem — renames update every referrer in the same change.
- Fenced code blocks carry a language; diagrams are ASCII in ```text fences.
- Tables for enumerable facts, prose for reasoning.
- No duplicated facts: if a fact appears twice, one occurrence becomes a link
  to the canonical home (`docs/guides/configuration.md` for env vars,
  `docs/reference/api-rest.md` for routes).
- Dates in ISO 8601 (`2026-09-14`). Prose in English.
- A doc and the code it describes change together: a new REST route without a
  `docs/reference/api-rest.md` entry is an incomplete change.
