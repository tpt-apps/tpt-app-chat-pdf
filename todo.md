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
- [ ] Single small-PDF upload + chat/question box, in-browser small model
- [ ] Free-tier upsell copy pointing at larger documents, multi-document
      libraries, exportable summaries, bigger model
- [ ] Browser-verified: mount, upload, ask a question, get a grounded answer

## Phase 3 — Paid desktop edition (cargo feature `pro`)
- [ ] `pro` gates: larger document support, multi-document libraries
      (cross-document retrieval), exportable summaries, bigger local model
- [ ] Pro UI: document library manager, cross-doc chat, "Export summary"
- [ ] Desktop shell via `tpt-appfront-webview`; Windows exe build + smoke test
      (bundling a bigger local model needs a download/install-size check)
- [ ] Gumroad product from apps.md listing copy ($29), exe upload, URL into
      registry — manual step, listing copy at apps.md §T2-7

## Phase 4 — Ship & measure
- [ ] Registry entry (`utility` category, `wasm: { entry: '/apps/chat-pdf/chat-pdf.js' }`)
      per the "Adding a finished app to the hub" runbook at the bottom of apps.md
- [ ] Verify hub card/detail/sitemap/runner in dev, then prod after deploy
- [ ] WASM size budget check on the shipped build (local-model weights are
      likely the dominant size cost — track separately from code size)
