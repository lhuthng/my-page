# Portfolio

Source for [portfolio.huuthangle.site](https://portfolio.huuthangle.site) — a static React site built with Bun and deployed to Cloudflare Pages.

---

## Tech Stack

| Library | Role |
|---|---|
| React 19 | UI framework |
| TypeScript | Type safety |
| Tailwind CSS v4 | Utility-first styling (via `bun-plugin-tailwind`) |
| GSAP 3 + `@gsap/react` | Scroll-driven and timeline animations |
| Lottie React | Lottie animation playback |
| RoughJS | Hand-drawn style canvas graphics |

No Vite. Bun's built-in bundler handles everything through a custom `build.ts` script.

---

## Local Development

```bash
bun install
bun dev
```

`bun dev` runs `bun --hot src/index.tsx`, which starts a dev server with hot module reloading. No config file needed — Bun picks up `tsconfig.json` automatically.

---

## Build

```bash
bun run build
```

This runs [`build.ts`](build.ts), which uses Bun's bundler API to scan `src/` for HTML entrypoints, run the Tailwind plugin, minify output, and emit everything to `dist/`. The previous `dist/` is wiped at the start of each build.

The script accepts CLI flags if you need to override defaults:

```bash
bun run build.ts --outdir=out --sourcemap=inline --no-minify
```

Run `bun run build.ts --help` to see all options.

---

## Deployment

Hosted on **Cloudflare Pages**. Any push to `master` that touches `portfolio/**` triggers an automatic build and deploy — no workflow step or extra configuration required.

The Cloudflare Pages project is configured with:
- **Build command:** `bun run build`
- **Output directory:** `dist`

There are no environment variables required at build time.