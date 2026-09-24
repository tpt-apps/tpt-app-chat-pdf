# tpt-app-chat-pdf

Offline "chat with your PDF" — on-device retrieval-augmented Q&A, no cloud,
no API key, no per-query cost. Spec: `1apps.md` §T2-7 in the
`tpt-electrician-nz-2` repo (hub slug `chat-pdf`, category `utility`).

- **Free edition** (this repo's default build, served at
  `tptsolutions.co.nz/tools/chat-pdf`): one small PDF, in-browser,
  wasm-bindgen `--target web` glue loaded by the hub's `WasmAppRunner`.
- **Pro edition** (`--features pro`, Phase 3): larger documents,
  multi-document libraries, exportable summaries, bigger local model —
  packaged as a Windows exe via `tpt-appfront-webview` and sold on Gumroad
  (listing copy: `GUMROAD.md`).

## Hub contract

The web repo's `WasmAppRunner` dynamic-imports the wasm-bindgen glue, awaits
its default export (wasm init), then calls the named export
`mount(container)` with the hub's container div. For standalone dev,
`trunk serve` + `index.html` provide a `#tpt-appfront-root` marker div that
the `#[wasm_bindgen(start)]` hook mounts into — the hub page has no such div,
so the two hosts never double-mount.

## Build

```sh
./build.ps1            # free edition -> web repo public/apps/chat-pdf/
./build.sh             # same on macOS/Linux
trunk serve            # standalone dev (mounts into #tpt-appfront-root)
```

`TPT_WEB_REPO` overrides the web repo destination; without it the bundle
lands in `dist/web/`.

## Layout

- `src/lib.rs` — hub contract glue (`mount` + `start`)
- `src/app.rs` — free-edition UI (tpt-appfront-dom)
- `engine/` — (Phase 1) target-independent RAG adapter, host-unit-testable
- `desktop/` — (Phase 3) native webview shell for the Pro edition
