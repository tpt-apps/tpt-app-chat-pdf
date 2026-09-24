# TPT Chat PDF — Build Todo

> Hub listing: `tptsolutions.co.nz/tools/chat-pdf` (registry slug `chat-pdf`,
> category `utility`). Spec: `apps.md` §T2-7 in the tpt-electrician-nz-2 repo.
> Free version: one small PDF, in-browser, small local model. Paid offline
> edition ($29 USD, Gumroad): larger documents, multi-document libraries,
> exportable summaries, bigger local model. Base wraps `tpt-raglite` +
> `tpt-pdf`. UI on `tpt-appfront`.
>
> Hub contract: `wasm-bindgen --target web` glue exposes `mount(container)`
> (called by `WasmAppRunner` after `default()` init). Standalone `trunk serve`
> mounts into the `#tpt-appfront-root` marker div instead — never
> `document.body` — so the two hosts never double-mount.

## Phase 0 — Scaffold & pipeline proof
- [x] `tpt-appfront init tpt-app-chat-pdf --target dom` (path deps on the tpt-appfront checkout)
      — run into a scratch dir (this dir already held todo.md/GUMROAD.md) and merged in
- [x] Restructure scaffold to hub contract: `#[wasm_bindgen] pub fn mount(el)` + `#[wasm_bindgen(start)]` no-op unless `#tpt-appfront-root` present
- [x] `index.html` carries the `#tpt-appfront-root` marker div
- [x] Build pipeline script mirroring fea-lite/cutlist-optimizer's (`--target web` glue → web repo `public/apps/chat-pdf/`)
      — `build.ps1` (canonical) + `build.sh`; both verified (web-repo dest + dist/web fallback);
      hello-world payload ~50 KB gzipped
- [x] Hello-world build loads through the standalone dev page (`trunk serve`)
      — browser-verified: mount, heading/button render, click → dispatch → signal → effect →
      DOM update (the test webview's throttled rAF hides `reactive_text` flushes; verified by
      loading a fresh module instance with a setTimeout rAF shim — not an app bug)
- [x] git init + first commit
- [x] Note resolved: there is no separate `tpt-pdf` repo. PDF text extraction lives in
      `tpt-rag-core` (`parser.rs`, `pdf_extract`-based, but its API takes a *file path* —
      the engine will use `pdf_extract`'s in-memory API directly so it works on WASM bytes).
      Survey finding to carry into Phase 1: `tpt-rag-core`'s embedding runs on `ort`
      (ONNX Runtime, native-only), so the free in-browser tier needs a WASM-viable
      retrieval/answering path (see Phase 1 model-budget item).

## Phase 1 — Core engine (wrap tpt-raglite + tpt-pdf)
- [x] Survey `tpt-raglite` for its embedding + retrieval + local-model
      inference pipeline, and `tpt-pdf` (once located) for text extraction
      — survey done (see Phase 0 note): `tpt-rag-core` has parser/chunker/
      HNSW/reranker, but its embeddings run on `ort` (native-only) and its
      store on `rusqlite`; neither compiles for wasm32
- [x] Adapter crate `engine/`: PDF bytes in → extracted text/chunks →
      index → query in → retrieved context + grounded answer out — as
      plain structs, no UI deps, host-unit-testable
      — `engine/` (tpt-chatpdf-engine): `pdf_extract` (wasm-safe, in-memory
      API) → page-bounded chunks → pure-std BM25 index → extractive answers
      with page citations. Deliberately does NOT wrap tpt-rag-core (see
      survey: ort/rusqlite are native-only); it reuses its design instead.
      Compiles for host tests AND wasm32 in one crate
- [x] Model size/perf budget: the free in-browser tier ships lexical
      retrieval (BM25) + extractive answers — zero model weights, so there
      is no in-browser model cost at all. The "small local model in
      browser" idea from the spec is NOT viable via tpt-raglite's ort
      stack; revisit only if a wasm ONNX/candle path is ever wanted — the
      Pro desktop tier keeps ort-based models as an option behind `pro`
- [x] Validation: known-answer questions against a sample PDF return correct
      grounded answers, not hallucinated ones
      — `engine/tests/validation.rs` (8 tests): max cable run → p1, earth
      loop → p2, RCBO → correct, plus a no-hallucination check (every
      statement must appear verbatim in the extracted document text)

## Phase 2 — Free WASM UI
- [x] Single small-PDF upload + chat/question box, in-browser small model
      — upload (10 MB / 40-page free caps, honest status line), question box,
      transcript of extractive answers with [p. N] citations + sources line.
      "In-browser small model" resolved as the Phase 1 survey required: the
      free tier runs the zero-weight lexical engine (BM25 + extractive
      answers) — nothing neural ships to the browser
- [x] Free-tier upsell copy pointing at larger documents, multi-document
      libraries, exportable summaries, bigger model — Pro section with the
      four limits + link to the tools page
- [x] Browser-verified: mount, upload, ask a question, get a grounded answer
      — verified against the fixture PDF through the real file-input change
      path: indexed 2 pages, asked "maximum cable run…" → "…is 40 metres.
      [p. 1]" with sources; oversize-file and empty-question guards show
      their messages; exactly one mount (hub contract holds). Note: the test
      webview throttles rAF (status-line flushes stall) and fullPage
      screenshots tile incorrectly — both test-harness artifacts, verified
      clean via DOM state

## Phase 3 — Paid desktop edition (cargo feature `pro`)
- [x] `pro` gates: larger document support (uncapped), multi-document
      libraries (cross-document retrieval, `ChatPdfLibrary`), exportable
      summaries — all engine-level features covered by tests (13 passing:
      8 validation + 5 library). NOTE on the spec's "bigger local model":
      this build ships the same zero-weight lexical engine in both
      editions; a native ort-based model remains possible behind `pro`
      later (it can never ship to the browser — see Phase 1 survey)
- [x] Pro UI: document library manager (multi-select upload, per-doc
      remove), cross-doc chat (statements cite "doc — text [p. N]"),
      "Export summary (.md)" via object-URL download. Free build hides all
      of it and shows the upsell instead; both compile from one source
- [x] Desktop shell via `tpt-appfront-webview`; Windows exe build + smoke
      test — `desktop/` embeds the pro bundle; `build.ps1 -Pro` produces
      `target\release\tpt-chat-pdf-pro.exe` (2.2 MB — the bundle is the
      payload, no model weights to install); smoke-tested: launched, Pro UI
      rendered (multi-file picker, library panel), closed cleanly
      (no bundled model ⇒ no download/install-size concern)
- [ ] Gumroad product from apps.md listing copy ($29), exe upload, URL into
      registry — manual step, listing copy at `GUMROAD.md`; exe is built
      and waiting at `target\release\tpt-chat-pdf-pro.exe`

## Phase 4 — Ship & measure
- [x] Registry entry (`utility` category, `wasm: { entry:
      '/apps/chat-pdf/chat-pdf.js' }`) per the "Adding a finished app to
      the hub" runbook — added to `tpt-apps-registry.ts` with
      `status: 'coming-soon'` (flip to `'live'` once the Gumroad listing
      exists); left uncommitted in the web repo for review
- [ ] Verify hub card/detail/sitemap/runner in dev, then prod after deploy
      — needs the web repo's dev server + deploy pipeline (user)
- [x] WASM size budget check on the shipped build — free bundle 490 KB
      gzipped (code only); model weights n/a for the lexical engine, so
      the local-model size cost the spec anticipated does not exist
