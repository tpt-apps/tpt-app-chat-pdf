//! TPT Chat PDF engine — target-independent on-device document Q&A.
//!
//! Pipeline: PDF bytes → per-page text → page-bounded chunks → BM25 lexical
//! index → extractive, grounded answers with page citations.
//!
//! Deliberately plain structs with no UI deps and no native-only
//! dependencies (no ONNX runtime, no SQLite), so the same crate is host
//! unit-testable and compiles for the wasm32 app.

mod answer;
mod chunk;
mod error;
mod extract;
mod index;

pub use answer::{Answer, Citation, Statement};
pub use chunk::Chunk;
pub use error::EngineError;

use index::BM25Index;

/// Default number of passages retrieved per question.
pub const DEFAULT_TOP_K: usize = 4;

/// An indexed document, ready for questions.
#[derive(Clone)]
pub struct ChatPdfEngine {
    index: BM25Index,
    page_count: usize,
}

impl ChatPdfEngine {
    /// Extracts and indexes one PDF from its raw bytes.
    pub fn from_pdf_bytes(bytes: &[u8]) -> Result<Self, EngineError> {
        let pages = extract::pages_from_pdf_bytes(bytes)?;
        let page_count = pages.len();
        let chunks = chunk::chunk_pages(&pages);
        if chunks.is_empty() {
            return Err(EngineError::NoExtractableText { page_count });
        }
        Ok(Self {
            index: BM25Index::build(chunks),
            page_count,
        })
    }

    /// Asks a question and returns grounded statements (with page
    /// citations) plus the retrieved passages. The answer is always
    /// extractive — every sentence comes verbatim from the document — so
    /// nothing can be hallucinated.
    pub fn ask(&self, question: &str, top_k: usize) -> Answer {
        answer::answer(&self.index, question, top_k)
    }

    /// Extractive overview of the whole document (top sentences across
    /// pages). The free tier shows a short version; the Pro edition exports
    /// the full summary.
    pub fn summary(&self, max_sentences: usize) -> Vec<Statement> {
        answer::summarize(&self.index, max_sentences)
    }

    pub fn page_count(&self) -> usize {
        self.page_count
    }

    pub fn chunk_count(&self) -> usize {
        self.index.len()
    }
}
