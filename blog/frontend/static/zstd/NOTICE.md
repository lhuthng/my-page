# Vendored zstd compressor

Used by the v86 snapshot studio to compress a `save_state()` blob in the
browser, so the server never spends CPU on it and the upload carries the
compressed bytes.

v86 needs no help decompressing these: `restore_state` sniffs the zstd frame
magic (`0xFD2FB528`) and unpacks internally, so the blob is stored and served
verbatim end to end.

## Contents

| File | Origin |
| --- | --- |
| `zstd.wasm` | `@bokuweb/zstd-wasm@0.0.27`, `dist/web/zstd.wasm`, copied unmodified |
| `zstd-compress-worker.js` | bundled from the same package plus the worker shim below |

Licences: `@bokuweb/zstd-wasm` is MIT; the underlying
[facebook/zstd](https://github.com/facebook/zstd) is dual BSD-3-Clause /
GPL-2.0.

## Why vendored rather than an npm dependency

`static/` is already how this project ships wasm — `libv86.js` and `v86.wasm`
are loaded by URL, and `vite.config.js` has no wasm plugin. Keeping the
compressor here means the admin-only studio pays for it and ordinary visitors
never download it.

## Rebuilding

The published package is ESM with extensionless imports, which browsers cannot
resolve, so it is bundled into a single module worker:

```bash
npm pack @bokuweb/zstd-wasm@0.0.27 && tar -xzf bokuweb-zstd-wasm-0.0.27.tgz
bun build worker-src.js --outfile=zstd-compress-worker.js --format=esm --target=browser --minify
cp package/dist/web/zstd.wasm .
```

`worker-src.js` imports `init`/`compress` from `package/dist/web/index.web.js`
and exposes the message contract below. The generated file is prefixed with a
provenance header by hand.

## Message contract

```js
worker.postMessage({ buffer, level: 19, wasmUrl: '/zstd/zstd.wasm' }, [buffer]);
// -> { type: 'started', rawSize }
// -> { type: 'done', compressed, compressedSize, rawSize, elapsedMs }
// -> { type: 'error', message }
```

Only the one-shot `ZSTD_compress` entry point is exported by this build, so
there is no incremental progress to report. Measured worst case is ~11 s for
72 MB of fully incompressible input at level 19 (peak ~420 MB RSS); real
machine states are mostly zero pages and finish in a fraction of that.
