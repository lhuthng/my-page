# GraphQL API Reference

Audience: admins and dashboard developers. Update trigger: a resolver change.

`POST /graphql` executes queries; `GET /graphql` serves the GraphQL
playground. The surface is **moderator+** (auth middleware in the router) and
read-only — the schema uses `EmptyMutation`.

GraphQL reads the SQLite pool directly rather than going through the service
traits; this is a documented exception — see
[../decisions/0001-backend-modularization.md](../decisions/0001-backend-modularization.md)
(§3.4, Option A).

## Queries

| Field | Returns | Description |
| --- | --- | --- |
| `users` | `[GqlUser]` | User listing with roles |
| `posts` | `PostConnection` | Paginated posts |
| `comments` | `CommentConnection` | Paginated comments |
| `media` | `MediaConnection` | Media listing |
| `series`, `seriesPosts` | `SeriesConnection` / series posts | Series data |
| `tags`, `categories` | `[GqlTag]` / `CategoryConnection` | Taxonomy |
| `dbStats` | `DbStats` | Row counts per table |
| `overview` | `GqlDashboardOverview` | Role counts, growth points, top content |
| `dashboardPosts`, `dashboardProjects` | connections | Management listings |
| `featuredPosts`, `featuredProjects` | `[GqlPost]` / `[GqlProject]` | Homepage picks |
| `postDetail`, `projectDetail`, `relatedPosts` | detail types | Single-item views |
| `checkSlug`, `checkProjectSlug` | availability | Slug checks |

The authoritative field list is `src/infrastructure/web/graphql/`
(`schema.rs` + `query/`); this table is a map. Types and row shapes live in
`types.rs` and `rows.rs`.
