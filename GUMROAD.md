# Gumroad listing

Everything needed to create/update the Gumroad product once this app is
built. Source copy is `apps.md` §T2-7 in the `tpt-electrician-nz-2` repo — if
that changes, update here too (and vice versa).

## Step-by-step: publish this listing

1. Build the Pro edition (`./build.ps1 -Pro`, once the build pipeline exists
   — see `todo.md` and mirror `tpt-app-fea-lite`/`tpt-app-cutlist-optimizer`'s
   pattern) — produces the packaged exe/zip in `release/`.
2. Log in to Gumroad → **New product** → **Digital product**.
3. Set the product name, price, and upload the packaged file (all below).
4. Paste the short description, bullets, and full description (below) into
   their respective fields.
5. Set category/tags (below) and add a cover image (see note below).
6. Publish, then copy the resulting product URL (`https://gum.co/...`).
7. Paste that URL into the `chat-pdf` entry's `gumroadUrl` field in
   `tpt-electrician-nz-2\src\lib\tpt-apps-registry.ts`, and set `price: '$29'`
   on the same entry.
8. Flip that entry's `status` to `'live'` once both the free demo and this
   Gumroad listing exist.
9. Commit and push the registry repo.

## File to upload

Not built yet. Once packaged, this will be the Pro exe/zip produced by the
app's `build.ps1 -Pro` (or equivalent) — see `todo.md`.

## Product name

```
TPT Chat PDF — Offline Document Q&A, No Cloud, No Per-Query Cost
```

## Price

**$29 USD, one-time purchase** (no subscription, no per-query API cost).

## Short description / summary field

```
Ask questions across your private documents without sending them to a cloud LLM. Pay once, use forever.
```

## Bullets / feature list

```
100% offline/local model
No API costs, ever
Multi-document libraries
Exportable summaries
One-time purchase
```

## Full description (long-form field)

```
TPT Chat PDF runs retrieval-augmented question-answering entirely on-device
against a local document set — no API key, no per-query cost, no document
ever leaves the machine.

Who it's for: researchers, consultants, and analysts repeatedly querying
large private document sets (due diligence files, thesis research corpora,
contract libraries). Cloud "chat with your PDF" tools charge per query or
per month and require sending confidential documents to a third-party LLM
API — a non-starter for due-diligence or client work under confidentiality
terms.

Pro edition (this download):
- Larger documents (free is capped to a small page count)
- Multi-document libraries with saved indexes
- Exportable answer summaries
- A bigger local model for better answer quality

Try the free single-small-PDF edition first, right in your browser, no
install: tptsolutions.co.nz/tools/chat-pdf

Requirements: Windows 10 (20H2+) or Windows 11. Uses the WebView2 runtime,
preinstalled on both — nothing extra to download.
```

## Category / tags

- Category: **Software > Developer Tools** (or Gumroad's closest equivalent)
- Tags: `local-llm`, `rag`, `document-qa`, `offline-ai`, `privacy-tools`,
  `windows`

## Cover image / thumbnail

Not created yet. A screenshot of a real question answered with a source
citation from a loaded document is the most convincing cover once built. No
stock/AI art.

## After publishing

1. Copy the resulting product URL (`https://gum.co/...`).
2. Paste it into the `gumroadUrl` field of the `chat-pdf` entry in
   `tpt-electrician-nz-2\src\lib\tpt-apps-registry.ts`, and set `price: '$29'`.
3. Set `status` to `'live'`.
4. Commit and push the registry repo.
