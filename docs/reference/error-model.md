# Error Model

Audience: backend contributors. Update trigger: a new error variant or a
change to the HTTP mapping.

## Shape

Each aggregate has its own error enum in `src/domain/errors/` (`PostError`,
`ProjectError`, `GameError`, `AudiobookError`, `MediaError`, `SeriesError`,
`UserError`, `AuthError`, `NewsletterError`, `DashboardError`, `SyncError`,
`MailError`). Handlers return `Result<_, AggregateError>` and Axum converts
the error into an HTTP response via the enum's `IntoResponse` impl.

Common variants across the enums:

| Variant | Meaning | Typical HTTP mapping |
| --- | --- | --- |
| `*NotFound` / `ProjectNotFound` | Entity missing | 404 |
| `Forbidden` | Role/ownership check failed | 403 |
| `Conflict` | Optimistic-concurrency or state conflict | 409 |
| `InvalidDemo` | Validation failure on demo/artifact input | 400 |
| `UploadFailed` | Multipart intake failure | 400 |
| `InternalError` | Unexpected server-side failure | 500 |

Cross-aggregate conversion is explicit: e.g. `GameError: From<ProjectError>`
wraps as `GameError::Project(_)`, so a v86 helper returning `ProjectError`
keeps its semantics when surfaced from a game route.

## Ownership guards

Ownership checks live in `web::api::support::ownership`
(`require_owner`, `require_can_delete`) and take the aggregate's pool/table
plus error constructors, so each aggregate keeps its own error type.

Authoritative list: the enums themselves in `src/domain/errors/`.
