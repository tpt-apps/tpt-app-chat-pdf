//! Free WASM edition UI (tpt-appfront-dom).
//!
//! Rendering architecture (mirrors the fea-lite / cutlist-optimizer split):
//! - the **input panel is mounted once** (`tpt_appfront_dom::mount`) — the
//!   question box never loses focus while typing; field values are read
//!   straight from the DOM when Ask is pressed;
//! - the **transcript re-renders fine-grained** via
//!   `tpt_appfront_dom::render` into a placeholder container, driven by the
//!   `turns` Signal;
//! - the **file input** needs the `FileList` (the DOM backend's `on_input`
//!   only carries the fake-path value), so a raw web-sys `change` listener
//!   is attached to the mounted input, matching fea-lite's canvas/slider
//!   precedent for widget kinds the DOM backend doesn't model.

use std::rc::Rc;

use tpt_appfront_core::{Signal, UITree};
use tpt_chatpdf_engine::{Answer, ChatPdfEngine};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Element, File, HtmlInputElement};

/// Free-tier caps (Pro lifts them — the copy below says so).
const MAX_FREE_BYTES: f64 = 10.0 * 1024.0 * 1024.0;
const FREE_PAGE_CAP: usize = 40;

/// One question + its grounded answer in the transcript.
#[derive(Clone)]
struct QaTurn {
    question: String,
    answer: Answer,
}

#[derive(Debug, Clone)]
enum Msg {
    Ask,
    Clear,
}

/// Mount the whole app into `container` (hub container div or the
/// `#tpt-appfront-root` marker).
pub fn mount_app(container: &Element) -> Result<(), JsValue> {
    let document = container
        .owner_document()
        .ok_or_else(|| JsValue::from_str("no owner document"))?;

    let status = Signal::new(String::from("Choose a PDF to ask questions about."));
    let doc: Signal<Option<Rc<ChatPdfEngine>>> = Signal::new(None);
    let turns: Signal<Rc<Vec<QaTurn>>> = Signal::new(Rc::new(Vec::new()));

    let ui: UITree<Msg> = UITree::container(|c| {
        c.heading(1, "TPT Chat PDF");
        c.text(
            "Ask questions about your PDF — answered entirely on this device, \
             no cloud, no API key, nothing uploaded.",
        );
        c.container(|c| {
            c.input("")
                .attr("type", "file")
                .attr("accept", "application/pdf,.pdf")
                .attr("id", "tpt-chatpdf-file");
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
        c.heading(2, "TPT Chat PDF Pro");
        c.text(
            "The free edition answers questions about one small PDF. Pro \
             ($29, one time — no subscription) lifts every limit: larger \
             documents, multi-document libraries with cross-document \
             questions, exportable summaries, and a bigger local model for \
             sharper answers.",
        );
        c.link(
            "https://tptsolutions.co.nz/tools/chat-pdf",
            "Get TPT Chat PDF Pro — $29 once, yours forever",
        );
    });

    let status_for_dispatch = status.clone();
    let doc_for_dispatch = doc.clone();
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
            let engine = doc_for_dispatch.get();
            let Some(engine) = engine else {
                status_for_dispatch
                    .set("Choose a PDF first — answers come from the document.".to_string());
                return;
            };
            let answer = engine.ask(&question, tpt_chatpdf_engine::DEFAULT_TOP_K);
            let mut next: Vec<QaTurn> = turns_for_dispatch.get().as_ref().clone();
            next.push(QaTurn {
                question,
                answer,
            });
            turns_for_dispatch.set(Rc::new(next));
            textarea.set_value("");
        }
        Msg::Clear => {
            turns_for_dispatch.set(Rc::new(Vec::new()));
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
            if turns.is_empty() {
                return UITree::container(|c| {
                    c.text(
                        "Answers appear here, quoted from the document with page numbers.",
                    );
                });
            }
            UITree::container(|c| {
                for turn in turns.iter() {
                    c.heading(3, format!("Q: {}", turn.question));
                    if turn.answer.statements.is_empty() {
                        c.text(
                            "No matching passage in this document — try different words.",
                        );
                    } else {
                        for stmt in &turn.answer.statements {
                            c.text(format!("{} [p. {}]", stmt.text, stmt.page));
                        }
                    }
                    let mut pages: Vec<String> =
                        turn.answer.passages.iter().map(|p| p.page.to_string()).collect();
                    pages.dedup();
                    c.text(format!("Sources: page {}", pages.join(", ")));
                }
                c.button("Clear conversation").on_click(Msg::Clear);
            })
        });
        // Whole-process root mount: keep the reconciler alive forever, like
        // the other handles in this app.
        std::mem::forget(tpt_appfront_dom::render(&results, view, Rc::clone(&dispatch))?);
    }

    // The DOM backend's on_input only carries the input's value; the file
    // selection itself needs a raw change listener (fea-lite canvas/slider
    // precedent).
    attach_file_listener(container, status, doc, turns)?;

    Ok(())
}

/// Wires the raw `change` listener that reads the selected PDF, enforces
/// the free-tier caps, extracts + indexes it off the event handler, and
/// keeps the status line honest the whole way.
fn attach_file_listener(
    container: &Element,
    status: Signal<String>,
    doc: Signal<Option<Rc<ChatPdfEngine>>>,
    turns: Signal<Rc<Vec<QaTurn>>>,
) -> Result<(), JsValue> {
    let input: HtmlInputElement = container
        .query_selector("#tpt-chatpdf-file")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into().ok())
        .ok_or_else(|| JsValue::from_str("file input missing"))?;

    let input_clone = input.clone();
    let on_change: Closure<dyn FnMut()> = Closure::wrap(Box::new(move || {
        let file: Option<File> = input_clone.files().and_then(|fl| fl.get(0));
        let Some(file) = file else { return };
        if file.size() > MAX_FREE_BYTES {
            status.set(format!(
                "That file is {:.1} MB — the free edition handles PDFs up to \
                 10 MB. Pro lifts the size cap.",
                file.size() / (1024.0 * 1024.0),
            ));
            return;
        }
        doc.set(None);
        turns.set(Rc::new(Vec::new()));
        status.set(format!("Reading “{}”…", file.name()));

        let status = status.clone();
        let doc = doc.clone();
        let file_name = file.name();
        let future = async move {
            let buffer = match JsFuture::from(file.array_buffer()).await {
                Ok(buffer) => buffer,
                Err(_) => {
                    status.set("Could not read that file — try again.".to_string());
                    return;
                }
            };
            let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
            status.set(format!("Indexing “{file_name}”…"));
            match ChatPdfEngine::from_pdf_bytes_capped(&bytes, FREE_PAGE_CAP) {
                Ok((engine, total_pages)) => {
                    let indexed = engine.page_count();
                    doc.set(Some(Rc::new(engine)));
                    if total_pages > indexed {
                        status.set(format!(
                            "Indexed “{file_name}” — first {indexed} of \
                             {total_pages} pages (free cap 40). Pro indexes it all."
                        ));
                    } else {
                        status.set(format!(
                            "Indexed “{file_name}” — {indexed} page(s). Ask away."
                        ));
                    }
                }
                Err(tpt_chatpdf_engine::EngineError::NoExtractableText { .. }) => {
                    status.set(
                        "No text found in that PDF — it looks like a scan. \
                         The free edition needs a text layer (Pro adds OCR)."
                            .to_string(),
                    );
                }
                Err(e) => {
                    status.set(format!("Could not read that PDF: {e}"));
                }
            }
        };
        wasm_bindgen_futures::spawn_local(future);
    }));
    input
        .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref())?;
    on_change.forget();
    Ok(())
}
