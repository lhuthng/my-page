# my-page

Personal platform for [Huu Thang](https://github.com/lhuthang): the blog at
[huuthangle.site](https://huuthangle.site) (SvelteKit SSR frontend + Rust/Axum
backend, Docker Compose on one VM behind nginx) and a separate static
[portfolio](portfolio/) site on Cloudflare Pages.

## Repo layout

```text
my-page/
|-- .github/        # CI/CD pipeline reference
|-- blog/           # the blog stack
|   |-- backend/    #   Rust/Axum API (see blog/backend/README.md)
|   |-- frontend/   #   SvelteKit SSR app
|   |-- docs/       #   feature design proposals
|   `-- docker-compose.yml, Makefile
|-- docs/           # repository documentation (architecture, guides, reference)
|-- portfolio/      # static React site, deployed to Cloudflare Pages
|-- ops/nginx/      # VM nginx sites managed from the repo
`-- scripts/emergency/  # manual deploy path
```

## Run it

```bash
cd blog
make setup            # create data dir, seed example .env files
# edit blog/backend/.env and blog/frontend/.env
docker compose up -d --build
```

Frontend at `http://localhost:5000`, backend bound to `127.0.0.1:3001`.
Standalone development: `make backend` and `make frontend`.
Details: [docs/guides/setup.md](docs/guides/setup.md).

## Configure it

Environment variables are documented exactly once in
[docs/guides/configuration.md](docs/guides/configuration.md).

## Deploy it

Pushes to `master` build both images and redeploy the VM via GitHub Actions;
`scripts/emergency/deploy.sh` is the manual path. See
[docs/guides/deployment.md](docs/guides/deployment.md).

## Documentation

Everything documentation-shaped lives under [docs/](docs/README.md):

- [Architecture](docs/architecture/overview.md) — system context, request
  flow, backend layer model, data model, deployment topology.
- [Guides](docs/guides/setup.md) — setup, configuration, development,
  deployment, operations, troubleshooting.
- [Reference](docs/reference/api-rest.md) — REST surface, GraphQL, media and
  storage, auth and roles, error model.
- [Decisions](docs/decisions/README.md) — architecture decision records.

Sub-project hubs: [blog/](blog/README.md),
[blog/backend/README.md](blog/backend/README.md),
[blog/frontend/README.md](blog/frontend/README.md),
[portfolio/](portfolio/README.md), [.github/](.github/README.md).

Contributing: commit conventions, formatting gates, and documentation rules
in [CONTRIBUTING.md](CONTRIBUTING.md).
