# Documentation Index

Everything documentation-shaped in this repository. Feature-level design
proposals are separate: [blog/docs/README.md](../blog/docs/README.md).

Start here if you are new: [architecture/overview.md](architecture/overview.md)
→ [guides/setup.md](guides/setup.md) → [guides/development.md](guides/development.md).

## Architecture

| Document | Contents |
| --- | --- |
| [architecture/overview.md](architecture/overview.md) | System context, components, the three frontend request paths |
| [architecture/backend.md](architecture/backend.md) | Backend layer model, module map, dependency rules, file budget |
| [architecture/frontend.md](architecture/frontend.md) | SvelteKit structure, SSR, proxy and media routing |
| [architecture/data-model.md](architecture/data-model.md) | Aggregates, tables, migration policy |
| [architecture/deployment.md](architecture/deployment.md) | VM, nginx, Docker, GHCR, DNS/TLS topology |

## Guides

| Document | Contents |
| --- | --- |
| [guides/setup.md](guides/setup.md) | Prerequisites, full-stack and standalone run paths |
| [guides/configuration.md](guides/configuration.md) | Every environment variable, per service — single source of truth |
| [guides/development.md](guides/development.md) | Make targets, migrations, lint/fmt gates, tests |
| [guides/deployment.md](guides/deployment.md) | CI/CD pipeline and the manual emergency path |
| [guides/operations.md](guides/operations.md) | Backups, sync-pull, R2↔fs migration, restore |
| [guides/troubleshooting.md](guides/troubleshooting.md) | Known failure modes and their symptoms |

## Reference

| Document | Contents |
| --- | --- |
| [reference/api-rest.md](reference/api-rest.md) | Complete REST surface, derived from the route tables |
| [reference/api-graphql.md](reference/api-graphql.md) | Admin GraphQL surface |
| [reference/media-and-storage.md](reference/media-and-storage.md) | ObjectStore, key layout, range serving |
| [reference/auth-and-roles.md](reference/auth-and-roles.md) | Token lifecycle, role tiers, guards |
| [reference/error-model.md](reference/error-model.md) | Per-aggregate error enums and HTTP mapping |

## Decisions

| Document | Contents |
| --- | --- |
| [decisions/README.md](decisions/README.md) | ADR index |
| [decisions/0001-backend-modularization.md](decisions/0001-backend-modularization.md) | Backend modularization + documentation architecture (implemented) |

## Sub-project hubs

[../blog/README.md](../blog/README.md) ·
[../blog/backend/README.md](../blog/backend/README.md) ·
[../blog/frontend/README.md](../blog/frontend/README.md) ·
[../portfolio/README.md](../portfolio/README.md) ·
[../.github/README.md](../.github/README.md) ·
[../ops/nginx/README.md](../ops/nginx/README.md) ·
[../scripts/emergency/README.md](../scripts/emergency/README.md)
