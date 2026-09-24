use crate::error::EngineError;

/// The text of one PDF page, 1-based `page` number for citations.
pub struct PageText {
    pub page: usize,
    pub text: String,
}

/// Extracts per-page text from raw PDF bytes. Page-wise extraction keeps
/// every chunk pinned to a page so citations stay exact.
pub fn pages_from_pdf_bytes(bytes: &[u8]) -> Result<Vec<PageText>, EngineError> {
    let page_texts = pdf_extract::extract_text_from_mem_by_pages(bytes)
        .map_err(|e| EngineError::Pdf(e.to_string()))?;
    Ok(page_texts
        .into_iter()
        .enumerate()
        .map(|(i, text)| PageText {
            page: i + 1,
            text,
        })
        .collect())
}
