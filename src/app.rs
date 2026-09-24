//! Free/Pro WASM UI (tpt-appfront-dom).
//!
//! Rendering architecture (mirrors the fea-lite / cutlist-optimizer split):
//! - the **input panel is mounted once** (`tpt_appfront_dom::mount`) — the
//!   question box never loses focus while typing; field values are read
//!   straight from the DOM when Ask is pressed;
//! - the **transcript and (pro) library panel re-render fine-grained** via
//!   `tpt_appfront_dom::render` into placeholder containers, driven by
//!   signals;
//! - the **file input** needs the `FileList` (the DOM backend's `on_input`
//!   only carries the fake-path value), so a raw web-sys `change` listener
//!   is attached to the mounted input, matching fea-lite's canvas/slider
//!   precedent for widget kinds the DOM backend doesn't model.
//!
//! Both editions run the same lexical engine; state is a
//! [`ChatPdfLibrary`] either way. Free = library of one, capped
//! (10 MB / 40 pages, replaced on each upload). Pro (`--features pro`) =
//! uncapped multi-document library, document-named citations, exportable
//! summaries — the paid edition's UI surface is cfg-gated.

use std::rc::Rc;

use tpt_appfront_core::{Signal, UITree};
use tpt_chatpdf_engine::ChatPdfLibrary;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Element, File, HtmlInputElement};

/// Free-tier caps (Pro lifts them — the copy below says so).
#[cfg(not(feature = "pro"))]
const MAX_FREE_BYTES: f64 = 10.0 * 1024.0 * 1024.0;
#[cfg(not(feature = "pro"))]
const FREE_PAGE_CAP: usize = 40;

/// One question + its grounded answer in the transcript.
#[derive(Clone)]
struct QaTurn {
    question: String,
    statements: Vec<tpt_chatpdf_engine::Statement>,
    pages: Vec<usize>,
}

#[derive(Debug, Clone)]
enum Msg {
    Ask,
    Clear,
    /// (pro) write the library summary to a downloaded Markdown file.
    #[cfg(feature = "pro")]
    ExportSummary,
    /// (pro) drop a document from the library by name.
    #[cfg(feature = "pro")]
    RemoveDoc(String),
}

/// Mount the whole app into `container` (hub container div or the
/// `#tpt-appfront-root` marker).
pub fn mount_app(container: &Element) -> Result<(), JsValue> {
    let document = container
        .owner_document()
        .ok_or_else(|| JsValue::from_str("no owner document"))?;

    let status = Signal::new(String::from("Choose a PDF to ask questions about."));
    let lib: Signal<Rc<ChatPdfLibrary>> = Signal::new(Rc::new(ChatPdfLibrary::new()));
    let turns: Signal<Rc<Vec<QaTurn>>> = Signal::new(Rc::new(Vec::new()));

    let ui: UITree<Msg> = UITree::container(|c| {
        c.heading(1, "TPT Chat PDF");
        c.text(
            "Ask questions about your PDFs — answered entirely on this device, \
             no cloud, no API key, nothing uploaded.",
        );
        c.container(|c| {
            let file = c
                .input("")
                .attr("type", "file")
                .attr("accept", "application/pdf,.pdf")
                .attr("id", "tpt-chatpdf-file");
            #[cfg(feature = "pro")]
            file.attr("multiple", "multiple");
            let _ = file;
        })
        .attr("class", "tpt-chatpdf-upload");
        c.heading(2, "Ask the document");
        c.container(|c| {
            c.textarea("")
                .attr("id", "tpt-chatpdf-question")
                .attr("placeholder", "e.g. What is the maximum cable run?")
                .attr("rows", "2");
        })
        .attr("class", "tpt-chatpdf-ask");
        c.button("Ask the document").on_click(Msg::Ask);
        // Transcript placeholder — `render` owns this container's children.
        c.container(|_| {})
            .attr("id", "tpt-chatpdf-results")
            .attr("class", "tpt-chatpdf-results");
        // (pro) library manager placeholder — `render` owns its children.
        #[cfg(feature = "pro")]
        c.container(|_| {})
            .attr("id", "tpt-chatpdf-library")
            .attr("class", "tpt-chatpdf-library");
        #[cfg(not(feature = "pro"))]
        c.heading(2, "TPT Chat PDF Pro");
        #[cfg(not(feature = "pro"))]
        c.text(
            "The free edition answers questions about one small PDF. Pro \
             ($29, one time — no subscription) lifts every limit: larger \
             documents, multi-document libraries with cross-document \
             questions, exportable summaries, and a bigger local model for \
             sharper answers.",
        );
        #[cfg(not(feature = "pro"))]
        c.link(
            "https://tptsolutions.co.nz/tools/chat-pdf",
            "Get TPT Chat PDF Pro — $29 once, yours forever",
        );
    });

    let status_for_dispatch = status.clone();
    let lib_for_dispatch = lib.clone();
    let turns_for_dispatch = turns.clone();
    let container_for_dispatch = container.clone();
    let dispatch: Rc<dyn Fn(Msg)> = Rc::new(move |msg| match msg {
        Msg::Ask => {
            let question_el = container_for_dispatch
                .query_selector("#tpt-chatpdf-question")
                .ok()
                .flatten()
                .and_then(|el| el.dyn_into::<web_sys::HtmlTextAreaElement>().ok());
            let Some(textarea) = question_el else {
                return;
            };
            let question = textarea.value().trim().to_string();
            if question.is_empty() {
                status_for_dispatch.set("Type a question first.".to_string());
                return;
            }
            let library = lib_for_dispatch.get();
            if library.is_empty() {
                status_for_dispatch
                    .set("Choose a PDF first — answers come from the document.".to_string());
                return;
            }
            // Free edition = library of one; Pro asks every document (the
            // library re-ranks across docs on one score scale).
            let library_answer = library.ask(&question, tpt_chatpdf_engine::DEFAULT_TOP_K);
            let single_doc = library.len() == 1;
            let statements = library_answer
                .statements
                .iter()
                .map(|s| tpt_chatpdf_engine::Statement {
                    // In a multi-document library the answer must say which
                    // document it came from; a single doc keeps the terse
                    // form.
                    text: if single_doc {
                        s.text.clone()
                    } else {
                        format!("{} — {}", s.document, s.text)
                    },
                    page: s.page,
                })
                .collect();
            let pages: Vec<usize> = library_answer
                .statements
                .iter()
                .map(|s| s.page)
                .collect();
            let mut next: Vec<QaTurn> = turns_for_dispatch.get().as_ref().clone();
            next.push(QaTurn {
                question,
                statements,
                pages,
            });
            turns_for_dispatch.set(Rc::new(next));
            textarea.set_value("");
        }
        Msg::Clear => {
            turns_for_dispatch.set(Rc::new(Vec::new()));
        }
        #[cfg(feature = "pro")]
        Msg::ExportSummary => export_summary(&lib_for_dispatch.get(), &status_for_dispatch),
        #[cfg(feature = "pro")]
        Msg::RemoveDoc(name) => {
            let mut library = (*lib_for_dispatch.get()).clone();
            if library.remove(&name) {
                status_for_dispatch.set(format!("Removed “{name}” from the library."));
            }
            lib_for_dispatch.set(Rc::new(library));
        }
    });

    tpt_appfront_dom::mount(container, &ui, Rc::clone(&dispatch))?;

    // Status line under the upload control (reactive fine-grained text).
    {
        let (text_node, text_handle) = tpt_appfront_dom::reactive_text(&document, status.clone())?;
        container.append_child(&text_node)?;
        // Whole-process root mount: forgetting is an explicit choice here,
        // matching the scaffold and the other hub apps.
        std::mem::forget(text_handle);
    }

    // Transcript: fine-grained re-render driven by the `turns` signal.
    {
        let results = container
            .query_selector("#tpt-chatpdf-results")
            .ok()
            .flatten()
            .ok_or_else(|| JsValue::from_str("transcript container missing"))?;
        let turns_for_view = turns.clone();
        let view: Rc<dyn Fn() -> UITree<Msg>> = Rc::new(move || {
            let turns = turns_for_view.get();
            UITree::container(|c| {
                if turns.is_empty() {
                    c.text(
                        "Answers appear here, quoted from the document with page numbers.",
                    );
                    return;
                }
                for turn in turns.iter() {
                    c.heading(3, format!("Q: {}", turn.question));
                    if turn.statements.is_empty() {
                        c.text(
                            "No matching passage in this document — try different words.",
                        );
                    } else {
                        for stmt in &turn.statements {
                            c.text(format!("{} [p. {}]", stmt.text, stmt.page));
                        }
                    }
                    let mut pages = turn.pages.clone();
                    pages.sort_unstable();
                    pages.dedup();
                    let pages = pages.iter().map(|p| p.to_string()).collect::<Vec<_>>();
                    c.text(format!("Sources: page {}", pages.join(", ")));
                }
                c.button("Clear conversation").on_click(Msg::Clear);
            })
        });
        // Whole-process root mount: keep the reconciler alive forever, like
        // the other handles in this app.
        std::mem::forget(tpt_appfront_dom::render(&results, view, Rc::clone(&dispatch))?);
    }

    // (pro) library manager: listing + remove + export, driven by `lib`.
    #[cfg(feature = "pro")]
    {
        let panel = container
            .query_selector("#tpt-chatpdf-library")
            .ok()
            .flatten()
            .ok_or_else(|| JsValue::from_str("library container missing"))?;
        let lib_for_view = lib.clone();
        let view: Rc<dyn Fn() -> UITree<Msg>> = Rc::new(move || {
            let library = lib_for_view.get();
            let names = library.document_names();
            UITree::container(|c| {
                c.heading(2, "Document library");
                if names.is_empty() {
                    c.text("No documents loaded — choose one or more PDFs above.");
                }
                for name in &names {
                    c.container(|row| {
                        row.text(name.clone());
                        row.button("Remove").on_click(Msg::RemoveDoc(name.clone()));
                    });
                }
                if !names.is_empty() {
                    c.button("Export summary (.md)").on_click(Msg::ExportSummary);
                }
            })
        });
        // Whole-process root mount: keep the reconciler alive forever.
        std::mem::forget(tpt_appfront_dom::render(&panel, view, Rc::clone(&dispatch))?);
    }

    // The DOM backend's on_input only carries the input's value; the file
    // selection itself needs a raw change listener (fea-lite canvas/slider
    // precedent).
    attach_file_listener(container, status, lib, turns)?;

    Ok(())
}

/// (pro) Writes every document's extractive summary to a downloaded
/// Markdown file via an object-URL anchor click.
#[cfg(feature = "pro")]
fn export_summary(library: &ChatPdfLibrary, status: &Signal<String>) {
    if library.is_empty() {
        status.set("Load a document before exporting a summary.".to_string());
        return;
    }
    let mut markdown = String::from("# TPT Chat PDF — document summaries\n\n");
    for (name, statements) in library.summarize(5) {
        markdown.push_str(&format!("## {name}\n\n"));
        for statement in statements {
            markdown.push_str(&format!("- {} (p. {})\n", statement.text, statement.page));
        }
        markdown.push('\n');
    }

    let blob = match web_sys::Blob::new_with_str_sequence(
        &js_sys::Array::from_iter([JsValue::from_str(&markdown)].iter()),
    ) {
        Ok(blob) => blob,
        Err(_) => {
            status.set("Could not build the summary file.".to_string());
            return;
        }
    };
    let url = match web_sys::Url::create_object_url_with_blob(&blob) {
        Ok(url) => url,
        Err(_) => {
            status.set("Could not stage the summary file.".to_string());
            return;
        }
    };
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let anchor = document
        .create_element("a")
        .ok()
        .and_then(|el| el.dyn_into::<web_sys::HtmlAnchorElement>().ok());
    let Some(anchor) = anchor else { return };
    anchor.set_href(&url);
    anchor.set_download("tpt-chat-pdf-summary.md");
    if let Some(body) = document.body() {
        let _ = body.append_child(&anchor);
        let _ = anchor.click();
        let _ = body.remove_child(&anchor);
    }
    let _ = web_sys::Url::revoke_object_url(&url);
    status.set(format!(
        "Exported summaries for {} document(s) to tpt-chat-pdf-summary.md.",
        library.len()
    ));
}

/// Reads the selected PDF(s), enforces the edition's caps, extracts +
/// indexes them off the event handler, and keeps the status line honest
/// the whole way. Free: one PDF at a time (the newest replaces it), capped.
/// Pro: every selected PDF joins the library, uncapped.
fn attach_file_listener(
    container: &Element,
    status: Signal<String>,
    lib: Signal<Rc<ChatPdfLibrary>>,
    turns: Signal<Rc<Vec<QaTurn>>>,
) -> Result<(), JsValue> {
    let input: HtmlInputElement = container
        .query_selector("#tpt-chatpdf-file")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into().ok())
        .ok_or_else(|| JsValue::from_str("file input missing"))?;

    let input_inner = input.clone();
    let on_change: Closure<dyn FnMut()> = Closure::wrap(Box::new(move || {
        let Some(files) = input_inner.files() else { return };
        let selected: Vec<File> = (0..files.length()).filter_map(|i| files.get(i)).collect();
        if selected.is_empty() {
            return;
        }
        #[cfg(not(feature = "pro"))]
        if selected[0].size() > MAX_FREE_BYTES {
            status.set(format!(
                "That file is {:.1} MB — the free edition handles PDFs up to \
                 10 MB. Pro lifts the size cap.",
                selected[0].size() / (1024.0 * 1024.0),
            ));
            return;
        }
        #[cfg(not(feature = "pro"))]
        {
            // Free: the library is exactly the newest PDF.
            lib.set(Rc::new(ChatPdfLibrary::new()));
            turns.set(Rc::new(Vec::new()));
        }
        #[cfg(feature = "pro")]
        let _ = &turns;

        let status = status.clone();
        let lib = lib.clone();
        let files: Vec<(String, File)> = selected
            .iter()
            .map(|f| (f.name(), f.clone()))
            .collect();
        let future = async move {
            let mut library = (*lib.get()).clone();
            let mut added = 0usize;
            let mut capped_note = String::new();
            for (name, file) in files {
                status.set(format!("Reading “{name}”…"));
                let buffer = match JsFuture::from(file.array_buffer()).await {
                    Ok(buffer) => buffer,
                    Err(_) => {
                        status.set(format!("Could not read “{name}” — try again."));
                        return;
                    }
                };
                let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
                status.set(format!("Indexing “{name}”…"));
                #[cfg(feature = "pro")]
                let outcome = library.add_pdf(&name, &bytes);
                #[cfg(not(feature = "pro"))]
                let outcome = library.add_pdf_capped(&name, &bytes, FREE_PAGE_CAP);
                match outcome {
                    Ok((indexed, total)) => {
                        added += 1;
                        if total > indexed {
                            capped_note = format!(
                                " (first {indexed} of {total} pages — Pro indexes it all)"
                            );
                        }
                    }
                    Err(tpt_chatpdf_engine::EngineError::NoExtractableText { .. }) => {
                        status.set(
                            "No text found in that PDF — it looks like a scan. \
                             The free edition needs a text layer (Pro adds OCR)."
                                .to_string(),
                        );
                        return;
                    }
                    Err(e) => {
                        status.set(format!("Could not read “{name}”: {e}"));
                        return;
                    }
                }
                // Free: stop after the first PDF regardless of extras.
                #[cfg(not(feature = "pro"))]
                if added == 1 {
                    break;
                }
            }
            lib.set(Rc::new(library.clone()));
            let noun = if library.len() == 1 { "document" } else { "documents" };
            status.set(format!("Indexed {added} {noun}. Ask away.{capped_note}"));
        };
        wasm_bindgen_futures::spawn_local(future);
    }));
    input
        .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref())?;
    on_change.forget();
    Ok(())
}
