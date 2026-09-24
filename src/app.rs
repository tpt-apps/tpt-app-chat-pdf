//! Free WASM edition UI (tpt-appfront-dom).
//!
//! Phase 0 pipeline proof: the hub/standalone host mounts this app, and a
//! button flip proves signals + dispatch + reactive re-render all work
//! through the wasm-bindgen glue. The PDF upload + chat UI replaces this in
//! Phase 2; the input panel will keep the same mounted-once / fine-grained
//! re-render split the fea-lite and cutlist-optimizer apps use.

use std::rc::Rc;

use tpt_appfront_core::{Signal, UITree};
use wasm_bindgen::prelude::*;
use web_sys::Element;

#[derive(Debug, Clone)]
enum Msg {
    /// Pipeline proof: flip the ready line through the dispatch loop.
    Ping,
}

/// Mount the whole app into `container` (hub container div or the
/// `#tpt-appfront-root` marker).
pub fn mount_app(container: &Element) -> Result<(), JsValue> {
    let ready = Signal::new(String::from("Loading the local engine…"));
    let pings = Signal::new(0i32);

    let ui: UITree<Msg> = UITree::container(|c| {
        c.heading(1, "TPT Chat PDF");
        c.text("Ask questions about your documents — entirely on this device, no cloud, no API key.");
        c.button("Check engine")
            .class("tpt-chatpdf-ping")
            .on_click(Msg::Ping);
    });

    let pings_for_effect = pings.clone();
    let ready_for_effect = ready.clone();
    let handle = tpt_appfront_core::create_effect(move || {
        match pings_for_effect.get() {
            0 => ready_for_effect.set(String::from("Loading the local engine…")),
            n => ready_for_effect.set(format!(
                "Engine pipeline live ({} ping{}). Phase 2 adds the upload + chat UI.",
                n,
                if n == 1 { "" } else { "s" }
            )),
        }
    });
    std::mem::forget(handle);

    let pings_for_dispatch = pings.clone();
    let dispatch: Rc<dyn Fn(Msg)> = Rc::new(move |msg| match msg {
        Msg::Ping => pings_for_dispatch.set(pings_for_dispatch.get() + 1),
    });

    tpt_appfront_dom::mount(container, &ui, dispatch)?;

    let (text_node, text_handle) = tpt_appfront_dom::reactive_text(
        &container.owner_document().expect("no owner document"),
        ready,
    )?;
    container.append_child(&text_node)?;
    // Root-mount lifetime: forgetting is an explicit choice here, matching
    // the scaffold and the other hub apps.
    std::mem::forget(text_handle);

    Ok(())
}
