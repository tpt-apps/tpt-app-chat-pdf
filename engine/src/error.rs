use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("PDF parsing failed: {0}")]
    Pdf(String),
    #[error(
        "no extractable text in {page_count} page(s) — the PDF is likely a scan without a text layer"
    )]
    NoExtractableText { page_count: usize },
}
